#include <cassert>
#include <iostream>
#include <map>              // For ordered maps (BTreeMap equivalent)
#include <stdexcept>
#include <string>
#include <unordered_map>    // For HashMap equivalent
#include <unordered_set>    // For HashMap equivalent
#include <vector>
#include <chrono>
#include <deque>
#include <filesystem>
#include <fstream>
#include <sstream>
#include "egraph_serialize.hpp"
#include "extractor.hpp"
#include <nlohmann/json.hpp>
#include "ordered_set.hpp"
#include "faster_greedy_dag.hpp"
using json = nlohmann::json;
using orderedset_c = tsl::ordered_set<ClassId>;
using orderedset_n = tsl::ordered_set<NodeId>;
using orderedset_c = tsl::ordered_set<ClassId>;


namespace fs = std::filesystem;


// --------------------------------------------------------------------
// Data Structures
// --------------------------------------------------------------------

// The Node structure. This mirrors the Rust struct.
// Serialization/deserialization (e.g. with a library like cereal or nlohmann::json)
// is not shown here.


// The Data structure. In Rust the nodes were stored in an IndexMap;
// here we use an unordered_map keyed by string. If ordering is required,
// you might use std::map or a custom container.

// --------------------------------------------------------------------
// remove_redundant_nodes Function
// --------------------------------------------------------------------
//
// This function removes redundant nodes from data by grouping nodes
// with the same parent children frequency signature within each eclass.
// The cost_func parameter is accepted (as in the Rust code) but is not used.
void remove_redundant_nodes(Data& data, const std::string& cost_func) {
    // This variable was declared in the Rust version but not used.
    // We keep it here in case you wish to use it later.
    std::unordered_map<uint32_t, std::unordered_map<std::string, bool>> eclass_hashes;

    // Step 1: Group node ids by their eclass.
    // eclass_collect maps each eclass (string) to a vector of node IDs (strings)
    std::unordered_map<ClassId, std::vector<NodeId>> eclass_collect;
    for (const auto& kv : data.nodes) {
        const ClassId& cid = kv.first;
        for (const auto& kw: kv.second){
            const NodeId& nid = kw.first;
            eclass_collect[cid].push_back(nid);
        }
        // const Node& node = kv.second;
    }

    // Step 2: For each eclass, group nodes by the frequency vector of their children.
    // grouped maps: key = frequency vector (ordered vector of (child, count) pairs)
    //          value = vector of node ids (strings) that share that frequency vector.
    for (const auto& ec_pair : eclass_collect) {
        const ClassId& eclass = ec_pair.first;
        const std::vector<NodeId>& node_ids = ec_pair.second;
        
        // Using std::map so that the key (a vector of pairs) is ordered.
        std::map<std::vector<std::pair<ClassId, size_t>>, std::vector<NodeId>> grouped;
        for (const auto& node_id : node_ids) {
            // Find the node; if not found, throw an error.
            // 从 node_id 中提取所属的 ClassId
            ClassId cid(node_id.value[0]);

            // 在外层映射中查找这个 ClassId
            auto outer_it = data.nodes.find(cid);
            if (outer_it == data.nodes.end()) {
                throw std::runtime_error(
                    std::string("Node not found for class id: ") + std::to_string(cid.return_value()));
            }

            // 在对应的内层映射中查找完整的 NodeId
            auto node_it = outer_it->second.find(node_id);
            if (node_it == outer_it->second.end()) {
                throw std::runtime_error(
                    std::string("Node not found for id: ") +
                    std::to_string(node_id.return_value()[0]) + ", " +
                    std::to_string(node_id.return_value()[1]));
            }
            const Node& node = node_it->second;
            
            // Copy the children vector (children_hashes)
            std::vector<ClassId> children_hashes = node.children;
            
            // 1. Count the occurrences of each child string.
            // Using std::map to get the keys in lexicographical order.
            std::map<ClassId, size_t> freq_map;
            for (const auto& elem : children_hashes) {
                ++freq_map[elem];
            }
            
            // 2. Convert freq_map into an ordered vector of (child, count) pairs.
            std::vector<std::pair<ClassId, size_t>> freq_vec;
            for (const auto& p : freq_map) {
                freq_vec.push_back(p);
            }
            
            // 3. Group by freq_vec.
            grouped[freq_vec].push_back(node_id);
        }
        
        // Step 3: For each group, if there are multiple node ids, keep the first one
        // and remove the rest from data.nodes.
        // std::map<std::vector<std::pair<NodeId, size_t>>, std::vector<NodeId>> grouped;
        for (const auto& group : grouped) {
            const std::vector<NodeId>& group_node_ids = group.second;
            if (group_node_ids.size() > 1) {
                // Iterate over the node ids; keep the first one (index 0), remove others.
                for (size_t idx = 0; idx < group_node_ids.size(); ++idx) {
                    const NodeId& item = group_node_ids[idx];
                    if (idx == 0) {
                        continue;  // keep the first node
                    } else {
                        // 从 item 中提取所属的 ClassId
                        ClassId cid(item.value[0]);

                        // 在外层映射中查找这个 ClassId
                        // 使用 at() 方法可以获得非 const 的引用（如果 key 不存在则抛出 std::out_of_range）
                        auto& inner_map = data.nodes.at(cid);

                        // 在内层映射中查找完整的 NodeId
                        auto inner_it = inner_map.find(item);
                        if (inner_it != inner_map.end()) {
                            inner_map.erase(inner_it);
                        } else {
                            throw std::runtime_error(
                                std::string("Node not found for id: ") +
                                std::to_string(item.return_value()[0]) + ", " +
                                std::to_string(item.return_value()[1]));
                        }
                    }
                }
            }
        }
    }
}

size_t egraph_partition(Data &data, float factor) {
    // Make a mutable copy of the nodes.
    auto mutable_nodes = data.nodes; // copy all nodes

    // Build a mapping from a child eclass to the list of parent node IDs.
    std::unordered_map<ClassId, std::vector<NodeId>> parents;

    // 遍历所有节点（注意：mutable_nodes 是嵌套的映射）
    for (const auto &outer_pair : mutable_nodes) {
        // outer_pair.first 是 ClassId，对应该 eclass的所有节点集合（外层键）
        // outer_pair.second 是一个 ordered_map<NodeId, Node>
        for (const auto &inner_pair : outer_pair.second) {
            // inner_pair.first 是该节点的 NodeId
            // inner_pair.second 是该节点的数据（Node）
            const NodeId &node_id = inner_pair.first;
            const Node &node = inner_pair.second;
            
            // 遍历该节点的 children（类型为 ClassId）
            for (const auto &child_class : node.children) {
                // 由于 node.children 存储的是 ClassId，我们直接使用 child_class
                // 如果想查找一个代表性节点（例如该 eclass中的第一个节点），可以这样做：
                auto child_outer_it = mutable_nodes.find(child_class);
                if (child_outer_it == mutable_nodes.end() || child_outer_it->second.empty()) {
                    throw std::runtime_error("Child node not found for eclass: " +
                                            std::to_string(child_class.return_value()));
                }
                // 此处我们不实际需要代表性节点，只是记录父节点的 NodeId
                // 将当前节点 node_id 作为父节点，记录到该子 eclass 对应的父节点列表中。
                if (parents.find(child_class) == parents.end()) {
                    parents[child_class] = std::vector<NodeId>();
                }
                parents[child_class].push_back(node_id);
            }
        }
    }



    // Measure the time required to compute the roots.
    auto start = std::chrono::high_resolution_clock::now();

    // Determine root nodes:
    std::vector<ClassId> root;
    for (const auto &outer_pair : mutable_nodes) {
        // outer_pair.first 是 ClassId，对应一个 eclass
        // outer_pair.second 是 tsl::ordered_map<NodeId, Node>，包含该 eclass 的所有节点
        for (const auto &inner_pair : outer_pair.second) {
            // inner_pair.first 的类型是 NodeId，inner_pair.second 的类型是 Node
            const NodeId &key = inner_pair.first;
            const Node &node = inner_pair.second;
            // 如果该节点的 eclass 在 parents 映射中没有父节点，则视为根节点
            if (parents.find(node.eclass) == parents.end()) {
                root.push_back(node.eclass);
                continue;
            }
        }
    }


    auto elapsed = std::chrono::high_resolution_clock::now() - start;
    auto millis = std::chrono::duration_cast<std::chrono::milliseconds>(elapsed).count();
    std::cout << "remove_redundant_nodes runtime-" << millis << " ms" << std::endl;

    // If more than one root is found, add a pseudo-root.
    if (root.size() > 1) {
        Node pseudo_root;
        
        // pseudo_root.op = "pseudo_root";
        // Check if "pseudo_root" is already in data.op
        auto it = std::find(data.op.begin(), data.op.end(), "pseudo_root");
        if (it == data.op.end()) {
            // Add "pseudo_root" to data.op
            data.op.push_back("pseudo_root");
            // Record the index of "pseudo_root" in data.op
            pseudo_root.op = data.op.size() - 1;
        } else {
            // Assign the existing index to pseudo_root.op
            pseudo_root.op = std::distance(data.op.begin(), it);
        }
        pseudo_root.children = root;
        pseudo_root.eclass = std::numeric_limits<unsigned int>::max()-1;
        pseudo_root.cost = 0.0f;

        NodeId new_root(std::numeric_limits<unsigned int>::max()-1,0);
        tsl::ordered_map<NodeId,Node> new_nodes;
        new_nodes[new_root] = pseudo_root;
        mutable_nodes[ClassId(std::numeric_limits<unsigned int>::max()-1)] = new_nodes;
        root = {ClassId(std::numeric_limits<unsigned int>::max()-1)};
    }

    // Build a mapping from each eclass to a vector of node IDs.
    // std::unordered_map<ClassId, std::vector<NodeId>> eclass_collect;
    // for (const auto &pair : mutable_nodes) {
    //     const NodeId &node_id = pair.first;
    //     const Node &node = pair.second;
    //     eclass_collect[node.eclass].push_back(node_id);
    // }

    std::unordered_map<ClassId, std::vector<NodeId>> eclass_collect;
    for (const auto& kv : data.nodes) {
        const ClassId& cid = kv.first;
        for (const auto& kw: kv.second){
            const NodeId& nid = kw.first;
            eclass_collect[cid].push_back(nid);
        }
        // const Node& node = kv.second;
    }

    // Determine the number of partitions.
    size_t partition_num = static_cast<size_t>(std::round(1.0f / factor));
    if (mutable_nodes.size() <= partition_num)
        throw std::runtime_error("Not enough nodes to partition (nodes.size() <= partition_num)");
    float num = static_cast<float>(mutable_nodes.size()) / partition_num;
    // std::cout << "num: " << num << std::endl;

    // Partitioning: traverse the graph and collect subgraphs.
    std::unordered_set<ClassId> visited;
    std::deque<ClassId> queue;
    std::vector<orderedset_c> subgraphs;
    orderedset_c current_subgraph;
    size_t current_count = 0;

    // Start the queue with the unique root.
    // NodeId root_node = mutable_nodes.at(root[0]).begin()->first;
    queue.push_back(root[0]);

    while (!queue.empty()) {
        ClassId class_id = queue.front();
        queue.pop_front();
        // // Get the eclass of the current node.
        // // 从 _node_id 中提取所属的 eclass：用 _node_id.value[0] 构造 ClassId
        // ClassId key_cid(_node_id.value[0]);
        // // 先在外层映射中查找对应的内层映射，然后在该内层映射中查找 _node_id 所对应的 Node
        // ClassId class_id = mutable_nodes.at(key_cid).at(_node_id).eclass;


        current_subgraph.insert(class_id);

        if (current_count >= num) {
            subgraphs.push_back(current_subgraph);
            current_subgraph.clear();
            current_count = 0;
            if (subgraphs.size() == partition_num)
                break;
        }

        // For this eclass, get the corresponding node IDs.
        auto it_ec = eclass_collect.find(class_id);
        if (it_ec != eclass_collect.end()) {
            const std::vector<NodeId> &class_nodes = it_ec->second;
            for (size_t idx = 0; idx < class_nodes.size(); ++idx) {
                current_count++;
                const NodeId &class_node = class_nodes[idx];
                // For each child of this node, if its eclass has not been visited, enqueue it.
                // 从 class_node 中提取 e-class，构造一个 ClassId 对象
                ClassId cid(class_node.value[0]);
                // 先获取该 e-class 对应的内层映射，再查找完整的 NodeId 对应的 Node
                const std::vector<ClassId> &children = mutable_nodes.at(cid).at(class_node).children;
                for (const ClassId &child_eclass : children) {
                    // ClassId child_eclass = mutable_nodes.at(child).eclass;
                    if (visited.find(child_eclass) == visited.end()) {
                        queue.push_back(child_eclass);
                        visited.insert(child_eclass);
                    }
                }
            }
        } else {
            throw std::runtime_error("class_id not found in eclass_collect: " + class_id.return_value());
        }
    }

    if (!current_subgraph.empty()) {
        subgraphs.push_back(current_subgraph);
    }

    // Verify that the union of subgraphs equals the set of all eclass keys.
    std::unordered_set<ClassId> union_subgraphs;
    for (const auto &sg : subgraphs) {
        union_subgraphs.insert(sg.begin(), sg.end());
    }
    std::unordered_set<ClassId> eclass_keys;
    for (const auto &pair : eclass_collect) {
        eclass_keys.insert(pair.first);
    }
    assert(union_subgraphs == eclass_keys);

    // Remove any existing "subgraph_" files from the "test" directory.
    for (const auto &entry : fs::directory_iterator("test")) {
        if (entry.is_regular_file()) {
            std::string filename = entry.path().filename().string();
            if (filename.rfind("subgraph_", 0) == 0) { // filename starts with "subgraph_"
                fs::remove(entry.path());
            }
        }
    }

    // Build subgraph maps and write each subgraph to a JSON file.
    // Here we use a vector of maps (subgraph_maps) where each map holds a set of nodes.
    std::vector<orderedmap_cid_nid_node> subgraph_maps;
    for (size_t idx = 0; idx < subgraphs.size(); ++idx) {
        const auto &subgraph = subgraphs[idx];
        orderedmap_cid_nid_node subgraph_map;
        // For every eclass in the subgraph, add all nodes in that eclass.
        for (const auto &class_id : subgraph) {
            auto it_ec = eclass_collect.find(class_id);
            if (it_ec != eclass_collect.end()) {
                // for (const auto &node_id : it_ec->second) {
                //     auto it_node = mutable_nodes.find(node_id);
                //     if (it_node != mutable_nodes.end()) {
                //         subgraph_map[node_id] = it_node->second;
                //     }
                // }
                auto outer_it = mutable_nodes.find(class_id);
                if (outer_it != mutable_nodes.end()) {
                    // 遍历属于该 eclass 的所有节点 id
                    for (const auto &node_id : it_ec->second) {
                        // 在内层映射中查找具体的 node
                        auto inner_it = outer_it->second.find(node_id);
                        if (inner_it != outer_it->second.end()) {
                            subgraph_map[class_id][node_id] = inner_it->second;
                        }
                    }
                }
            }
        }

        // First pass: record children that are not present in this subgraph.
        std::unordered_map<NodeId, std::vector<ClassId>> to_remove;
        std::unordered_map<ClassId, std::vector<NodeId>> subgraph_parents;

        // 辅助函数：检查 subgraph_map 中是否存在某个 eclass
        auto exists_in_subgraph = [&subgraph_map](const ClassId &cid) -> bool {
            // 直接检查外层 key是否存在
            return subgraph_map.find(cid) != subgraph_map.end();
        };

        // 或者，如果有必要遍历每个节点来确认其 eclass（一般不需要）
        // auto exists_in_subgraph = [&subgraph_map](const ClassId &cid) -> bool {
        //     for (const auto &outer_pair : subgraph_map) {
        //         for (const auto &inner_pair : outer_pair.second) {
        //             if (inner_pair.second.eclass == cid)
        //                 return true;
        //         }
        //     }
        //     return false;
        // };

        // 遍历 subgraph_map 中所有节点（内层映射中的每个元素）
        for (const auto &outer_pair : subgraph_map) {
            // outer_pair.first 是 eclass（ClassId）
            // outer_pair.second 是该 eclass 下的节点映射：tsl::ordered_map<NodeId, Node>
            for (const auto &inner_pair : outer_pair.second) {
                const NodeId &node_key = inner_pair.first;
                const Node &node = inner_pair.second;
                // 遍历该节点的 children，注意 children 的元素类型为 ClassId
                for (const auto &child_eclass : node.children) {
                    // 判断 subgraph_map 中是否存在任一节点，其 eclass 等于 child_eclass
                    if (!exists_in_subgraph(child_eclass)) {
                        // 记录缺失的子 eclass
                        to_remove[node_key].push_back(child_eclass);
                    } else {
                        // 如果存在，则将当前节点 node_key 作为父节点记录到 subgraph_parents 中，
                        // 键为 child_eclass，值为一个 NodeId 列表。
                        subgraph_parents[child_eclass].push_back(node_key);
                    }
                }
            }
        }

        // Second pass: for each node, remove invalid children.
        for (auto &outer_pair : subgraph_map) {
            // outer_pair.first 是 eclass（ClassId）
            // outer_pair.second 是该 eclass 下的节点映射：tsl::ordered_map<NodeId, Node>
            for (auto &inner_pair : outer_pair.second) {
                // inner_pair.first 是 NodeId，inner_pair.second 是 Node（使用引用以便修改）
                NodeId current_node_id = inner_pair.first;
                // Node &node = inner_pair.second;
                Node &node = const_cast<Node&>(inner_pair.second);
                if (to_remove.find(current_node_id) != to_remove.end()) {
                    const auto &children_to_remove = to_remove[current_node_id];
                    // node.children.erase(
                    //     std::remove_if(node.children.begin(), node.children.end(),
                    //                 [&children_to_remove](const NodeId &c) {
                    //                     return std::find(children_to_remove.begin(),
                    //                                         children_to_remove.end(), c) != children_to_remove.end();
                    //                 }),
                    //     node.children.end());
                    node.children.erase(
                        std::remove_if(node.children.begin(), node.children.end(),
                                    [&children_to_remove](const ClassId &c) {
                                        return std::find(children_to_remove.begin(), children_to_remove.end(), c) != children_to_remove.end();
                                    }),
                        node.children.end());
                }
            }
        }





        // Determine the set of "root" eclasses in the subgraph.
        orderedset_c subgraph_root;
        for (const auto &pair : subgraph_map) {
            // pair.first 是一个 ClassId，代表该 eclass
            const ClassId &key_eclass = pair.first;
            if (subgraph_parents.find(key_eclass) == subgraph_parents.end()) {
                subgraph_root.insert(key_eclass);
            }
        }


        // If there are multiple roots, create a pseudo-root.
        if (subgraph_root.size() > 1) {
            Node pseudo_root;
            // pseudo_root.op = "pseudo_root_" + std::to_string(idx);
            // Convert the unordered_set into a vector.

            auto it = std::find(data.op.begin(), data.op.end(), "pseudo_root");
            if (it == data.op.end()) {
                // Add "pseudo_root" to data.op
                data.op.push_back("pseudo_root");
                // Record the index of "pseudo_root" in data.op
                pseudo_root.op = data.op.size() - 1;
            } else {
                // Assign the existing index to pseudo_root.op
                pseudo_root.op = std::distance(data.op.begin(), it);
            }

            pseudo_root.children.assign(subgraph_root.begin(), subgraph_root.end());
            pseudo_root.eclass = std::numeric_limits<unsigned int>::max()-1;
            pseudo_root.cost = 0.0f;

            NodeId new_root( static_cast<unsigned int>(std::numeric_limits<unsigned int>::max() - 2 - idx) ,0);
            tsl::ordered_map<NodeId,Node> new_nodes;
            new_nodes[new_root] = pseudo_root;
            // subgraph_map[ static_cast<unsigned int>(std::numeric_limits<unsigned int>::max() - 2 - idx) ] = new_nodes;
            // subgraph_root = { static_cast<unsigned int>(std::numeric_limits<unsigned int>::max() - 2 - idx) };
            subgraph_map[ClassId(std::numeric_limits<unsigned int>::max()-2-idx)] = new_nodes;
            subgraph_root = { ClassId(std::numeric_limits<unsigned int>::max()-2-idx) };
        }


        // if (root.size() > 1) {
        //     Node pseudo_root;
            
        //     // pseudo_root.op = "pseudo_root";
        //     // Check if "pseudo_root" is already in data.op
        //     auto it = std::find(data.op.begin(), data.op.end(), "pseudo_root");
        //     if (it == data.op.end()) {
        //         // Add "pseudo_root" to data.op
        //         data.op.push_back("pseudo_root");
        //         // Record the index of "pseudo_root" in data.op
        //         pseudo_root.op = data.op.size() - 1;
        //     } else {
        //         // Assign the existing index to pseudo_root.op
        //         pseudo_root.op = std::distance(data.op.begin(), it);
        //     }
        //     pseudo_root.children = root;
        //     pseudo_root.eclass = std::numeric_limits<unsigned int>::max()-1;
        //     pseudo_root.cost = 0.0f;

        //     NodeId new_root(std::numeric_limits<unsigned int>::max()-1,0);
        //     tsl::ordered_map<NodeId,Node> new_nodes;
        //     new_nodes[new_root] = pseudo_root;
        //     mutable_nodes[std::numeric_limits<unsigned int>::max()-1] = new_nodes;
        //     root = {std::numeric_limits<unsigned int>::max()-1};
        // }


        assert(subgraph_root.size() == 1);

        // Create new Data for this subgraph.
        Data new_data;
        // We must convert subgraph_map (std::map) to the same type as Data::nodes.
        for (const auto &pair : subgraph_map) {
            new_data.nodes[pair.first] = pair.second;
        }
        // Set the root_eclasses vector.
        std::cout << "Root of subgraph: " << (*subgraph_root.begin()).return_value() << std::endl;
        // new_data.root_eclasses.push_back(subgraph_map.at(*subgraph_root.begin()).eclass);
        new_data.root_eclasses.assign(subgraph_root.begin(), subgraph_root.end());
        subgraph_maps.push_back(subgraph_map);

        // Serialize new_data to JSON.
        json j = new_data;
        std::string new_file_content = j.dump(4);  // pretty-print with indent=4

        // Write to file "test/subgraph_<idx>.json".
        fs::path file_path = fs::path("test") / ("subgraph_" + std::to_string(idx) + ".json");
        std::ofstream ofs(file_path);
        if (!ofs)
            throw std::runtime_error("Unable to write file: " + file_path.string());
        ofs << new_file_content;
    }

    return partition_num;
}

int main() {
    try {
        // --- Step 1. Define the filename and build the file path ---
        std::string filename = "smoothe_artifact/dataset_new/set/test.json";
        // Alternatively, you might use:
        // std::string filename = "smoothe_artifact/dataset/rover/box_filter_5iteration_egraph.json";

        // Build a path relative to the current working directory.
        std::filesystem::path file_path = std::filesystem::current_path() / filename;

        // --- Step 2. Read the file content ---
        std::ifstream infile(file_path);
        if (!infile) {
            throw std::runtime_error("Unable to open file: " + file_path.string());
        }
        std::stringstream buffer;
        buffer << infile.rdbuf();
        std::string file_content = buffer.str();
        infile.close();


        // --- Step 3. Parse JSON into Data ---
        json j = json::parse(file_content);
        Data data = j.get<Data>();

        // --- Step 4. Remove redundant nodes ---
        remove_redundant_nodes(data, "dag");

        // --- Step 5. Serialize Data back to JSON and write to file ---
        json j_new = data;
        std::string new_file_content = j_new.dump(4);  // Pretty-print with an indent of 4 spaces

        // Ensure the "test" directory exists.
        std::filesystem::create_directories("test");

        std::string out_filename = "test/remove_redundant.json";
        std::ofstream outfile(out_filename);
        if (!outfile) {
            throw std::runtime_error("Unable to open output file: " + out_filename);
        }
        outfile << new_file_content;
        outfile.close();

        // --- Step 6. Partition the e-graph ---
        size_t partition_num = egraph_partition(data, 0.33f);
        std::cout << "Partition number: " << partition_num << std::endl;

        // --- Step 7. Load the total e-graph from the JSON file ---
        // If you wish to add extra context to errors, you can wrap this in a try/catch.
        EGraph total_egraph = EGraph::from_json_file(out_filename);
        FasterGreedyDagExtractor extractor;
        ExtractionResult result = extractor.extract(total_egraph, total_egraph.root_eclasses);
        result.check(total_egraph);

        // for (const auto &pair: total_egraph.nodes) {
        //     std::cout << "Node: " << pair.first.str();
        //     std::cout << " op:" << pair.second.op;
        //     std::cout << " children";
        //     for (const auto &child : pair.second.children) {
        //         std::cout << " " << child.str();
        //     }
        //     std::cout << " cost:" << pair.second.cost ;
        //     std::cout << " EClass: " << pair.second.eclass.str() << std::endl;
        // }

        // for (const auto &pair: result.choices) {
        //     std::cout << "Node: " << pair.first.str() << " Choice: " << pair.second.str() << std::endl;
        // }

        // result.tree_cost(&total_egraph, &total_egraph.root_eclasses);
        auto tree = result.tree_cost(total_egraph, total_egraph.root_eclasses);
        auto dag = result.dag_cost(total_egraph, total_egraph.root_eclasses);
        
        std::cout << "Tree cost: " << tree << std::endl;
        std::cout << "DAG cost: " << dag << std::endl;

        // std::cout << "SerializedEGraph loaded successfully." << std::endl;


    }
    catch (const std::exception &ex) {
        std::cerr << "Error: " << ex.what() << std::endl;
        return 1;
    }
    return 0;
}