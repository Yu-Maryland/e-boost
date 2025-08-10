#ifndef EGRAPH_HPP
#define EGRAPH_HPP

#include <cassert>
#include <exception>
#include <functional>
#include <iostream>
#include <memory>
#include <optional>
#include <stdexcept>
#include <string>
#include <vector>
#include <nlohmann/json.hpp>
#include <unordered_map>    // For HashMap equivalent
#include <unordered_set>    // For HashMap equivalent
#include "ordered_map.hpp"
using json = nlohmann::json;


struct NodeId {

    // std::array<int, 2> value;

    // // Constructors
    // NodeId(int v1, int v2) : value({v1, v2}) {}

    // // Return the underlying array.
    // const std::array<int, 2>& str() const { return value; }

    // // Equality and ordering (based on the array content)
    // bool operator==(const NodeId &other) const {
    //     return value == other.value;
    // }
    // bool operator<(const NodeId &other) const {
    //     return value < other.value;
    // }

    // std::array<int, 2> value;
    std::array<unsigned int, 2> value;

    // Constructors
    NodeId(unsigned int v1, unsigned int v2) : value{v1, v2} {}

    // Return the underlying string.
    const std::array<unsigned int, 2>& return_value() const { return value; }

    // Equality and ordering (based on the string content)
    bool operator==(const NodeId &other) const {
        return value == other.value;
    }
    
    bool operator<(const NodeId &other) const {
        if (value[0] != other.value[0]) {
            return value[0] < other.value[0];
        }
        return value[1] < other.value[1];
    }
};

struct ClassId {
    // std::shared_ptr<std::string> value;
    unsigned int value;

    // Constructors
    // ClassId(const std::string &s) : value(std::make_shared<std::string>(s)) {}
    // ClassId(std::string &&s) : value(std::make_shared<std::string>(std::move(s))) {}
    // ClassId(const char* s) : value(std::make_shared<std::string>(s)) {}
    ClassId(unsigned int s) : value(s) {}

    // Return the underlying string.
    // const std::string& str() const { return *value; }
    unsigned int return_value() const { return value; }

    // Equality and ordering (based on the string content)
    // bool operator==(const ClassId &other) const {
    //     return *value == *other.value;
    // }
    // bool operator<(const ClassId &other) const {
    //     return *value < *other.value;
    // }
    bool operator==(const ClassId &other) const {
        return value == other.value;
    }
    bool operator<(const ClassId &other) const {
        return value < other.value;
    }
};





// Provide std::hash specializations so these types can be used in unordered containers.
namespace std {
    template<>
    struct hash<NodeId> {
        std::size_t operator()(const NodeId &nid) const {
            return std::hash<int>()(nid.value[0]) ^ (std::hash<int>()(nid.value[1]) << 1);
        }
    };

    template<>
    // struct hash<ClassId> {
    //     std::size_t operator()(const ClassId &cid) const {
    //         return std::hash<std::string>()(*cid.value);
    //     }
    // };
    struct hash<ClassId> {
        std::size_t operator()(const ClassId &cid) const {
            return std::hash<unsigned int>()(cid.value);
        }
    };
} // namespace std

// --- Forward declarations ---
struct Node;
struct Class;
struct ClassData;


using orderedmap_classid_class = tsl::ordered_map<ClassId,Class>;
using orderedmap_nodeid_node = tsl::ordered_map<NodeId, Node>;
using orderedmap_cid_nid_node = tsl::ordered_map<ClassId, tsl::ordered_map<NodeId,Node>>;
using orderedmap_classid_classdata = tsl::ordered_map<ClassId, ClassData>;

// A simple “cost” type. In Rust this is a NotNan<f64> (i.e. a non-NaN f64).
// In C++ we simply use a double and assume that the caller will not set NaN.

// --- Node ---
//
// Represents a node in the e-graph.
struct Node {
    unsigned int op;
    std::vector<ClassId> children;
    ClassId eclass;
    double cost;
    

    // Constructor with a default cost (equivalent to one() in Rust)
    Node(const unsigned int &op_, const ClassId &eclass_, double cost_ = 1.0)
        : op(op_), eclass(eclass_), cost(cost_) {}

    // Default constructor
    Node() : op(), children(), eclass(std::numeric_limits<unsigned int>::max()), cost(1.0) {}

    bool is_leaf() const {
        return children.empty();
    }
};

// --- Class ---
//
// A grouping of nodes that share the same e-class.
struct Class {
    ClassId id;
    std::vector<NodeId> nodes;

    Class() : id(std::numeric_limits<unsigned int>::max()), nodes() {}
    Class(const ClassId &id_) : id(id_), nodes() {}
};



// --- ClassData ---
//
// Additional data associated with an e-class.
struct ClassData {
    // We use std::optional<std::string> to mimic Option<String> in Rust.
    std::optional<std::string> typ;
};


// struct Node_new {
//     std::string op;
//     std::vector<std::string> children;
//     std::string eclass;
//     float cost;
// };

struct Data {
    orderedmap_cid_nid_node nodes;
    std::vector<ClassId> root_eclasses;
    std::vector<std::string> op;
};


inline void to_json(json& j, const Node& n) {
    j = json::object();
    j["op"] = n.op;

    // 将 children 从 std::vector<NodeId> 转换为 std::vector<std::string>
    std::vector<unsigned int> children_str;
    for (const auto &child : n.children) {
        children_str.push_back(child.return_value());
    }
    j["children"] = children_str;
    
    j["eclass"] = n.eclass.return_value();
    j["cost"] = n.cost;
}

// inline void from_json(const json& j, Node& n) {
//     j.at("op").get_to(n.op);
//     j.at("children").get_to(n.children);
//     j.at("eclass").get_to(n.eclass);
//     j.at("cost").get_to(n.cost);
// }

inline void from_json(const json& j, Node& n) {
    j.at("op").get_to(n.op);
    
    // 先将 children 读入到 std::vector<std::string> 中
    // std::vector<std::string> children_str;
    // j.at("children").get_to(children_str);
    // n.children.clear();
    // for (const auto &child : children_str) {
    //     n.children.emplace_back(child);
    // }

    std::vector<unsigned int> children_val;
    j.at("children").get_to(children_val);
    n.children.clear();
    for (const auto &child_val : children_val) {
        n.children.emplace_back(child_val); // 调用 ClassId(unsigned int)
    }
    

    const json& eclass_json = j.at("eclass");
    unsigned int eclass_val;
    if (eclass_json.is_number_integer()) {
        // 如果 JSON 中 eclass 是整数，则直接获取
        eclass_val = eclass_json.get<unsigned int>();
    } else if (eclass_json.is_string()) {
        // 如果是字符串，则尝试将字符串转换为 unsigned int
        std::string eclass_str = eclass_json.get<std::string>();
        try {
            size_t pos = 0;
            // 使用 std::stoul 进行转换，同时获取解析结束的位置
            unsigned long ul = std::stoul(eclass_str, &pos, 10);
            // 检查是否整个字符串都被正确解析
            if (pos != eclass_str.size()) {
                throw std::runtime_error("eclass 字符串包含非法字符: " + eclass_str);
            }
            // 检查转换结果是否超出 unsigned int 的范围
            if (ul > std::numeric_limits<unsigned int>::max()) {
                throw std::runtime_error("eclass 字符串表示的数字超出范围: " + eclass_str);
            }
            eclass_val = static_cast<unsigned int>(ul);
        } catch (const std::invalid_argument& e) {
            throw std::runtime_error("eclass 字符串无法转换为数字: " + eclass_str);
        } catch (const std::out_of_range& e) {
            throw std::runtime_error("eclass 字符串表示的数字超出范围: " + eclass_str);
        }
    } else {
        throw std::runtime_error("eclass 的类型必须是整数或表示整数的字符串");
    }

    n.eclass = ClassId(eclass_val);
    
    j.at("cost").get_to(n.cost);

}

// 序列化 NodeId 为字符串
inline void to_json(json &j, const NodeId &nid) {
    j = nid.return_value();
}

// 反序列化 NodeId，从字符串构造 NodeId
inline void from_json(const json &j, NodeId &nid) {
    auto arr = j.get<std::array<unsigned int, 2>>();
    nid = NodeId(arr[0], arr[1]);
}

// 同理，对 ClassId 进行转换
inline void to_json(json &j, const ClassId &cid) {
    j = cid.return_value();
}

inline void from_json(const json &j, ClassId &cid) {
    cid = ClassId(j.get<unsigned int>());
}


// to_json and from_json for Data
// inline void to_json(json& j, const Data& d) {
//     json nodes_obj = json::object();
//     for (const auto &entry : d.nodes) {
//         nodes_obj[ std::to_string(entry.first.return_value()) ] = entry.second;
//     }
    
//     j = json::object();
//     j["nodes"] = nodes_obj;
//     j["root_eclasses"] = d.root_eclasses;
//     j["op"] = d.op;
// }

inline void to_json(json& j, const Data& d) {
    json nodes_obj = json::object();

    // 遍历每个 e-class
    for (const auto &class_entry : d.nodes) {
        // 将外层键（ClassId）转换为字符串
        std::string class_key = std::to_string(class_entry.first.return_value());
        json inner_obj = json::object();
        // 遍历该 e-class 内所有节点（内层映射）
        for (const auto &node_entry : class_entry.second) {
            const NodeId &nid = node_entry.first;
            const Node &node = node_entry.second;
            // 使用 NodeId 的第二个值作为键
            std::string node_key = std::to_string(nid.return_value()[1]);
            
            // 先将 Node 转为 json（调用 to_json(Node)），然后加入额外的 "nid" 属性
            json node_json = node;
            node_json["nid"] = nid;  // 这里会调用已定义的 to_json(NodeId), 返回数组形式

            inner_obj[node_key] = node_json;
        }
        nodes_obj[class_key] = inner_obj;
    }

    j = json::object();
    // 加入一个空的 class_data 对象
    j["class_data"] = json::object();
    j["nodes"] = nodes_obj;
    j["root_eclasses"] = d.root_eclasses;  // 假设 to_json(ClassId) 已经定义为返回数字
    j["op"] = d.op;
}


inline void from_json(const json& j, Data& d) {

    d.nodes.clear();
    const json& nodes_json = j.at("nodes");
    // std::cout << nodes_json << std::endl;
    // int kkkk=0;
    for (const auto& outer : nodes_json.items()) {
        unsigned int num1 = std::stoul(outer.key());
        for (const auto& inner : outer.value().items()) {
            unsigned int num2 = std::stoul(inner.key());
            NodeId nid(num1, num2);
            Node node = inner.value().get<Node>();
            d.nodes[ClassId(num1)].emplace(nid, node);
        }
    }

    // j.at("root_eclasses").get_to(d.root_eclasses);

    // 先读取为 std::vector<std::string>
    // std::vector<std::string> root_eclasses_str = j.at("root_eclasses").get<std::vector<std::string>>();
    std::vector<unsigned int> root_eclasses;
    const json& root_eclasses_json = j.at("root_eclasses");

    for (const auto &elem : root_eclasses_json) {
        unsigned int value;
        if (elem.is_number_integer()) {
            // 如果元素是整数类型，直接获取
            value = elem.get<unsigned int>();
        } else if (elem.is_string()) {
            // 如果元素是字符串类型，尝试转换为数字
            std::string s = elem.get<std::string>();
            try {
                size_t pos = 0;
                unsigned long ul = std::stoul(s, &pos, 10);
                // 检查整个字符串是否被正确解析
                if (pos != s.size()) {
                    throw std::runtime_error("root_eclasses 字符串包含非法字符: " + s);
                }
                // 检查转换结果是否超出 unsigned int 范围
                if (ul > std::numeric_limits<unsigned int>::max()) {
                    throw std::runtime_error("root_eclasses 字符串表示的数字超出范围: " + s);
                }
                value = static_cast<unsigned int>(ul);
            } catch (const std::invalid_argument&) {
                throw std::runtime_error("root_eclasses 字符串无法转换为数字: " + s);
            } catch (const std::out_of_range&) {
                throw std::runtime_error("root_eclasses 字符串表示的数字超出范围: " + s);
            }
        } else {
            throw std::runtime_error("root_eclasses 的元素类型必须是整数或表示整数的字符串");
        }
        root_eclasses.push_back(value);
    }


    d.root_eclasses.clear();
    // 将每个字符串构造成一个 ClassId 对象，并加入到 d.root_eclasses 中
    for (const auto &s : root_eclasses) {
        d.root_eclasses.emplace_back(s);
    }

    const json& op_json = j.at("op");
    d.op = op_json.get<std::vector<std::string>>();
}




// --- EGraph ---
//
// The main data structure holding the nodes, e-classes, and associated data.

class EGraph {
public:
    // Members corresponding to the Rust fields.
    orderedmap_cid_nid_node nodes;
    std::vector<ClassId> root_eclasses;
    orderedmap_classid_classdata class_data;
    std::vector<std::string> op;

    // Cache for grouping nodes by their e-class.
    // Once computed, this cache is never updated.
    mutable std::optional<orderedmap_classid_class> classes_cache;

    EGraph() = default;

    // Adds a new node to the e-graph.
    // Throws an exception if a node with the same id already exists.
    // void add_node(const NodeId &node_id, const Node &node) {
    //     auto result = nodes.emplace(node_id, node);
    //     if (!result.second) {
    //         // Here we throw an exception. In a more sophisticated system, you might want
    //         // to provide a more detailed error message.
    //         throw std::runtime_error("Duplicate node with id: [" + std::to_string(node_id.value[0]) + ", " + std::to_string(node_id.value[1]) + "]");
    //     }
    // }

    void add_node(const NodeId &node_id, const Node &node) {
        // 先获得外层映射对应的内层映射，外层键为 ClassId(node_id.value[0])
        auto &inner_map = nodes[ClassId(node_id.value[0])];
        // 然后在内层映射中插入节点，键类型为 NodeId
        auto result = inner_map.emplace(node_id, node);
        if (!result.second) {
            throw std::runtime_error("Duplicate node with id: [" +
                                    std::to_string(node_id.value[0]) + ", " +
                                    std::to_string(node_id.value[1]) + "]");
        }
    }


    // Returns the e-class id of the node identified by node_id.
    // Throws if the node_id is not found.
    const ClassId& nid_to_cid(const NodeId &node_id) const {
        return this->operator[](node_id).eclass;
    }

    // Returns the e-class corresponding to the node identified by node_id.
    const Class& nid_to_class(const NodeId &node_id) const {
        return (*this)[ this->operator[](node_id).eclass ];
    }

    // Groups the nodes by their e-class.
    // This is computed only once and then cached. Subsequent modifications to the
    // e-graph will not be reflected.
    const orderedmap_classid_class& classes() const {
        if (!classes_cache.has_value()) {
            orderedmap_classid_class cls;
            // 外层遍历：键为 ClassId，值为内部的节点映射
            for (const auto &outer : nodes) {
                // outer.first 是 ClassId，outer.second 是 tsl::ordered_map<NodeId, Node>
                for (const auto &inner : outer.second) {
                    const NodeId &node_id = inner.first;
                    const Node &node = inner.second;
                    const ClassId &cid = node.eclass;
                    // 如果 cls 中还没有这个 e-class，则添加一个新条目
                    if (cls.find(cid) == cls.end()) {
                        cls[cid] = Class(cid);
                    }
                    cls[cid].nodes.push_back(node_id);
                }
            }
            classes_cache = std::move(cls);
        }
        return *classes_cache;
    }


    // --- Indexing Operators ---
    //
    // Provides a way to access nodes and classes by their id.
    const Node& operator[](const NodeId &node_id) const {
        // 第一步：根据 node_id 的第一个数字构造 ClassId
        ClassId cid(node_id.value[0]);
        
        // 在外层映射中查找这个 ClassId
        auto outer_it = nodes.find(cid);
        if (outer_it == nodes.end()) {
            throw std::runtime_error("No nodes with class id: " + std::to_string(cid.return_value()));
        }
        
        // 在对应的内层映射中查找完整的 NodeId
        const auto &inner_map = outer_it->second;
        auto inner_it = inner_map.find(node_id);
        if (inner_it == inner_map.end()) {
            throw std::runtime_error("No node with id: [" +
                                    std::to_string(node_id.value[0]) + ", " +
                                    std::to_string(node_id.value[1]) + "]");
        }
        
        return inner_it->second;
    }



    const Class& operator[](const ClassId &class_id) const {
        const auto &cls = classes();
        auto it = cls.find(class_id);
        if (it == cls.end()) {
            throw std::runtime_error("No class with id: " + class_id.return_value());
        }
        return it->second;
    }

    // --- (De)serialization functions ---
    //
    // In Rust these were provided via serde. In C++ you might use a library
    // like nlohmann/json. For now, we leave these as stubs.
    // You would need to implement these using your favorite JSON library.
    // 在 EGraph 类内添加或实现这个静态函数
    static EGraph from_json_file(const std::string &path) {
        // 打开文件
        std::ifstream file(path);
        if (!file.is_open()) {
            throw std::runtime_error("无法打开文件: " + path);
        }

        // 将文件内容读入 JSON 对象
        json j;
        try {
            file >> j;
        } catch (const json::parse_error &e) {
            throw std::runtime_error("解析 JSON 时出错（文件 " + path + "）: " + std::string(e.what()));
        }


        // 将 JSON 转换为 Data 结构（Data 已经提供了 from_json 实现）
        Data d;
        try {
            d = j.get<Data>();
        } catch (const json::type_error &e) {
            throw std::runtime_error("将 JSON 转换为 Data 结构时出错: " + std::string(e.what()));
        }

        // // 构造 EGraph 对象，并将 Data 中的数据赋值给 EGraph 的对应成员
        EGraph eg;
        eg.nodes = std::move(d.nodes);
        eg.root_eclasses = std::move(d.root_eclasses);
        eg.op = std::move(d.op);
        // // 注意：eg.class_data 保持为空，eg.classes_cache 会在首次调用时懒加载构建

        return eg;
    }

    // // Writes the EGraph to a JSON file in a pretty-printed format.
    // void to_json_file(const std::string &path) const {
    //     // Implement file reading and JSON parsing here.
    //     throw std::runtime_error("from_json_file not implemented");
    // }

    // 在 EGraph 类中实现 to_json_file 成员函数
    void to_json_file(const std::string &path) const {
        // 构造一个 Data 对象，用于存储 EGraph 中需要序列化的部分
        Data d;
        d.nodes = nodes;              // 注意：Data 中的 nodes 类型为 orderedmap_nodeid_node
        d.root_eclasses = root_eclasses;
        d.op = op;

        // 利用已定义的 to_json 函数将 Data 对象转换为 JSON 对象
        json j = d;

        // 打开文件（以写入模式），如果文件无法打开则抛出异常
        std::ofstream file(path);
        if (!file.is_open()) {
            throw std::runtime_error("无法打开文件以写入: " + path);
        }

        // 将 JSON 对象以格式化的形式写入文件，缩进设置为4个空格
        file << j.dump(4);

        // 关闭文件
        file.close();
    }

    void test_round_trip() const {
        // Implement a round-trip test for (de)serialization.
        throw std::runtime_error("test_round_trip not implemented");
    }

};



#endif  // EGRAPH_HPP
