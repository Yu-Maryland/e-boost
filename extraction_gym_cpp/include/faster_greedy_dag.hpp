#include <algorithm>
#include <deque>
#include <limits>
#include <memory>
#include <optional>
#include <stdexcept>
#include <string>
#include <unordered_map>
#include <unordered_set>
#include <vector>
#include "extractor.hpp"

// -----------------------
// UniqueQueue 模板类
// -----------------------
template <typename T>
class UniqueQueue {
public:
    UniqueQueue() = default;

    // 如果 t 不在队列中，则将其加入队列
    void insert(const T &t) {
        if (set.insert(t).second) {
            queue.push_back(t);
        }
    }

    // 批量加入元素（接受迭代器）
    template <typename Iterator>
    void extend(Iterator begin, Iterator end) {
        for (auto it = begin; it != end; ++it) {
            insert(*it);
        }
    }

    // 弹出队首元素，返回 std::optional<T>
    std::optional<T> pop() {
        if (queue.empty()) return std::nullopt;
        T front = queue.front();
        queue.pop_front();
        set.erase(front);
        return front;
    }

    bool empty() const {
        return queue.empty();
    }

public:
    std::unordered_set<T> set;
    std::deque<T> queue;
};

// -----------------------
// FasterGreedyDagExtractor 类
// -----------------------
class FasterGreedyDagExtractor : public Extractor {
public:
    FasterGreedyDagExtractor() = default;

    // 实现 Extractor 的抽象方法 extract
    virtual ExtractionResult extract(const EGraph &egraph,
                                     const std::vector<ClassId> & /*roots*/) const override {
        // Lambda 用于从 NodeId 获取所属 e-class
        auto n2c = [&egraph](const NodeId &nid) -> const ClassId& {
            return egraph.nid_to_cid(nid);
        };

        // 构造父节点映射：每个 e-class 对应一个包含其所有父节点的 vector
        tsl::ordered_map<ClassId, std::vector<NodeId>> parents;
        auto classes_map = egraph.classes();  // 得到所有 e-class 的映射
        
        // std::cout << "classes_map: ";
        // for (const auto &pair : classes_map) {
        //     std::cout << pair.first.str() << " ";
        // }
        // std::cout << std::endl;
        for (const auto &pair : classes_map) {
            parents[pair.first] = std::vector<NodeId>();
        }

        // 构造唯一队列，初始时将叶子节点入队
        UniqueQueue<NodeId> analysis_pending;
        for (const auto &pair : classes_map) {
            const Class &cls = pair.second;
            for (const auto &node_id : cls.nodes) {
                const Node &node = egraph[node_id];
                // 对于每个子节点，将当前节点记为其父节点
                for (const ClassId &child : node.children) {
                    parents[child].push_back(node_id);
                }
                // 如果是叶子节点，加入待处理队列
                if (node.is_leaf()) {
                    analysis_pending.insert(node_id);
                }
            }
        }

        ExtractionResult result;
        // 存储每个 e-class 的最佳成本集
        std::unordered_map<ClassId, CostSet> costs;

        // 自底向上处理：只处理那些所有子节点成本已经计算好的节点
        while (!analysis_pending.empty()) {
            std::optional<NodeId> maybe_node_id = analysis_pending.pop();
            if (!maybe_node_id.has_value())
                break;
            NodeId node_id = maybe_node_id.value();
            const ClassId &class_id = n2c(node_id);
            const Node &node = egraph[node_id];

            // 检查当前节点的所有子节点所属 e-class 是否都已经在 costs 中计算好
            bool all_children_in_costs = true;
            for (const auto &child_cid : node.children) {
                // const ClassId &child_cid = n2c(child);
                if (costs.find(child_cid) == costs.end()) {
                    all_children_in_costs = false;
                    break;
                }
            }
            if (!all_children_in_costs) {
                continue;
            }

            // 如果当前 e-class 已有成本，则 prev_cost 为之前的成本；否则设为无穷大
            double prev_cost = std::numeric_limits<double>::infinity();
            auto it = costs.find(class_id);
            if (it != costs.end()) {
                prev_cost = it->second.total;
            }

            // 计算当前节点的成本集
            CostSet cost_set = calculate_cost_set(egraph, node_id, costs, prev_cost);

            if (cost_set.total < prev_cost) {
                costs[class_id] = cost_set;
                // 将所有该 e-class 的父节点加入队列
                if (parents.find(class_id) != parents.end()) {
                    analysis_pending.extend(parents[class_id].begin(), parents[class_id].end());
                }
            }
        }

        // 将最终成本集中每个 e-class 选择的 node 写入 ExtractionResult
        for (const auto &p : costs) {
            result.choose(p.first, p.second.choice);
        }

        return result;
    }

    // 实现 clone 方法，要求派生类提供拷贝构造函数
    virtual FasterGreedyDagExtractor* clone() const override {
        return new FasterGreedyDagExtractor(*this);
    }

protected:
    // -----------------------
    // CostSet 结构体：存储某 e-class 的成本信息
    // -----------------------
    struct CostSet {
        std::unordered_map<ClassId, Cost> costs; // 每个 e-class 的成本（共享的部分只计一次）
        Cost total;                              // 总成本
        NodeId choice;                           // 当前 e-class 的最佳选择 node

        //std::numeric_limits<unsigned int>::max() for None
        //std::numeric_limits<unsigned int>::max()-1 for pseudo_root
        CostSet() : total(std::numeric_limits<double>::infinity()), choice(NodeId(std::numeric_limits<unsigned int>::max(),std::numeric_limits<unsigned int>::max())) {}
    };

    // -----------------------
    // calculate_cost_set 函数
    // -----------------------
    // 计算给定节点的成本集：如果共享节点只计一次成本，则返回一个成本集
    static CostSet calculate_cost_set(const EGraph &egraph, const NodeId &node_id, const std::unordered_map<ClassId, CostSet> &costs, Cost best_cost) {
        CostSet cs;
        const Node &node = egraph[node_id];
        const ClassId &cid = egraph.nid_to_cid(node_id);

        // 如果是叶子节点，直接返回
        if (node.children.empty()) {
            cs.costs[cid] = node.cost;
            cs.total = node.cost;
            cs.choice = node_id;
            return cs;
        }

        // 获取子节点所属的唯一 e-class 列表
        std::vector<ClassId> childrens_classes=node.children;
        // for (const auto &child : node.children) {
        //     childrens_classes.push_back(egraph.nid_to_cid(child));
        // }
        std::sort(childrens_classes.begin(), childrens_classes.end());
        childrens_classes.erase(std::unique(childrens_classes.begin(), childrens_classes.end()),
                                 childrens_classes.end());

        // 必须至少有一个子节点
        if (childrens_classes.empty()) {
            cs.total = std::numeric_limits<double>::infinity();
            cs.choice = node_id;
            return cs;
        }

        // 取第一个子类的成本集
        auto first_it = costs.find(childrens_classes[0]);
        if (first_it == costs.end()) {
            cs.total = std::numeric_limits<double>::infinity();
            cs.choice = node_id;
            return cs;
        }
        const CostSet &first_cost = first_it->second;

        // 如果当前 e-class 在子节点列表中，或者子节点只有一个且 (node.cost + first_cost.total) 大于 best_cost，则不能更优
        bool contains_cid = std::find(childrens_classes.begin(), childrens_classes.end(), cid) != childrens_classes.end();
        if (contains_cid || (childrens_classes.size() == 1 && (node.cost + first_cost.total > best_cost))) {
            cs.total = std::numeric_limits<double>::infinity();
            cs.choice = node_id;
            return cs;
        }

        // 找出成本集中条目数最多的子节点对应的 e-class
        size_t max_size = 0;
        ClassId id_of_biggest = childrens_classes[0];
        for (const auto &child_cid : childrens_classes) {
            auto it = costs.find(child_cid);
            if (it != costs.end()) {
                size_t size = it->second.costs.size();
                if (size > max_size) {
                    max_size = size;
                    id_of_biggest = child_cid;
                }
            }
        }

        // 复制最大的成本集，并合并其它子节点的成本
        std::unordered_map<ClassId, Cost> result_map;
        auto it_biggest = costs.find(id_of_biggest);
        if (it_biggest != costs.end()) {
            result_map = it_biggest->second.costs;  // 拷贝
        }
        for (const auto &child_cid : childrens_classes) {
            if (child_cid == id_of_biggest) continue;
            auto it_child = costs.find(child_cid);
            if (it_child != costs.end()) {
                for (const auto &pair : it_child->second.costs) {
                    result_map.insert(pair);
                }
            }
        }

        // 检查结果中是否已有当前 e-class 的成本
        bool already_contains = (result_map.find(cid) != result_map.end());
        result_map[cid] = node.cost;

        // 如果已经存在则返回无穷大成本，否则计算总成本
        double result_cost = 0.0;
        if (already_contains) {
            result_cost = std::numeric_limits<double>::infinity();
        } else {
            for (const auto &pair : result_map) {
                result_cost += pair.second;
            }
        }

        cs.costs = std::move(result_map);
        cs.total = result_cost;
        cs.choice = node_id;
        return cs;
    }
};