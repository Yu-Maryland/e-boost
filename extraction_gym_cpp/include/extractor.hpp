#ifndef EXTRACTION_HPP
#define EXTRACTION_HPP

#include <cassert>
#include <cmath>
#include <limits>
#include <stdexcept>
#include <unordered_map>
#include <unordered_set>
#include <vector>
#include <memory>
#include <numeric>

// --------------------------------------------------------------------
// Constants and Utility Functions
// --------------------------------------------------------------------

// We assume that Cost is defined as double in our C++ code.
using Cost = double;
class EGraph;

// The INFINITY constant in Rust (using NotNan) is translated here as:
inline constexpr Cost COST_INFINITY = std::numeric_limits<double>::infinity();

// Allowance for floating point values to be considered equal.
inline constexpr double EPSILON_ALLOWANCE = 0.00001;

// --------------------------------------------------------------------
// The Extractor Abstract Base Class
// --------------------------------------------------------------------
//
// In Rust this is a trait with a method extract; in C++ we define it
// as an abstract class with a pure virtual function.
class Extractor {
public:
    /// Given an e-graph and a list of root e-class ids, extract a result.
    virtual class ExtractionResult extract(const EGraph& egraph,
                                           const std::vector<ClassId>& roots) const = 0;

    virtual ~Extractor() = default;

    /// Helper method to return a unique_ptr (similar to Rust's boxed)
    std::unique_ptr<Extractor> boxed() const {
        // Note: This default implementation requires that your concrete class
        // has a copy constructor. You may want to override this method.
        return std::unique_ptr<Extractor>(this->clone());
    }

protected:
    /// Clone function (to be overridden by concrete subclasses)
    virtual Extractor* clone() const = 0;
};

// --------------------------------------------------------------------
// A Generic MapGet Helper
// --------------------------------------------------------------------
//
// In Rust the MapGet trait is implemented for several map types. In C++ we
// can write a helper function template that returns a pointer to the value
// (or nullptr if the key is not present).
template <typename Map, typename Key>
inline const typename Map::mapped_type* map_get(const Map& m, const Key& key) {
    auto it = m.find(key);
    return (it != m.end()) ? &it->second : nullptr;
}

// --------------------------------------------------------------------
// ExtractionResult
// --------------------------------------------------------------------
//
// This structure contains the choices (a mapping from ClassId to NodeId)
// that were selected during extraction.
using orderedmap_classid_nodeid = tsl::ordered_map<ClassId, NodeId>;
class ExtractionResult {
public:
    // Using an ordered map (if order is important) or unordered_map.
    // Here we use std::unordered_map; if you require ordering, consider std::map.
    orderedmap_classid_nodeid choices;

    ExtractionResult() = default;

    explicit ExtractionResult(const orderedmap_classid_nodeid& choices_)
        : choices(choices_) {}

    // ----------------------------------------------------------------
    // Check the extraction result for correctness.
    // ----------------------------------------------------------------
    void check(const EGraph& egraph) const {
        // The e-graph should have at least one root.
        assert(!egraph.root_eclasses.empty());

        // All roots should be selected.
        for (const auto& cid : egraph.root_eclasses) {
            if (choices.find(cid) == choices.end()) {
                // throw std::logic_error("ExtractionResult::check: Missing choice for root " + cid.return_value());
                throw std::logic_error("ExtractionResult::check: Missing choice for root " + std::to_string(cid.return_value()));
            }
        }

        // No cycles should be present.
        if (!find_cycles(egraph, egraph.root_eclasses).empty()) {
            throw std::logic_error("ExtractionResult::check: Cycle detected in extraction choices.");
        }

        // For every (ClassId, NodeId) in choices, ensure that the node’s eclass matches.
        for (const auto& pair : choices) {
            const ClassId& cid = pair.first;
            const NodeId& nid = pair.second;
            const Node& node = egraph[nid];
            if (!(node.eclass == cid)) {
                throw std::logic_error("ExtractionResult::check: Node eclass does not match its ClassId.");
            }
        }

        // All nodes that the roots depend upon should be selected.
        std::vector<ClassId> todo = egraph.root_eclasses; // copy
        std::unordered_set<ClassId> visited;
        while (!todo.empty()) {
            ClassId cid = todo.back();
            todo.pop_back();

            if (!visited.insert(cid).second) {
                continue;
            }
            if (choices.find(cid) == choices.end()) {
                // throw std::logic_error("ExtractionResult::check: Missing choice for dependent class " + cid.return_value());
                throw std::logic_error("ExtractionResult::check: Missing choice for root " + std::to_string(cid.return_value()));
            }
            const NodeId& node_id = choices.at(cid);
            const Node& node = egraph[node_id];
            for (const ClassId& child : node.children) {
                todo.push_back(child);
            }
        }
    }

    // ----------------------------------------------------------------
    // Record a new extraction choice.
    // ----------------------------------------------------------------
    void choose(const ClassId& class_id, const NodeId& node_id) {
        choices.insert({ class_id, node_id });
    }

    // ----------------------------------------------------------------
    // Find cycles in the extraction choices.
    // Returns a vector of ClassIds that are involved in a cycle.
    // ----------------------------------------------------------------
    std::vector<ClassId> find_cycles(const EGraph& egraph,
                                     const std::vector<ClassId>& roots) const {
        // Status map: ClassId -> Status (Doing or Done).
        // std::unordered_map<ClassId, enum Status>
        
        tsl::ordered_map<ClassId,enum Status> status;
        std::vector<ClassId> cycles;
        for (const auto& root : roots) {
            cycle_dfs(egraph, root, status, cycles);
        }
        return cycles;
    }

    // ----------------------------------------------------------------
    // Compute the tree cost of the extraction.
    // This cost is computed by summing the cost of the selected node
    // for each e-class, recursively.
    // ----------------------------------------------------------------
    Cost tree_cost(const EGraph& egraph,
                   const std::vector<ClassId>& roots) const {
        // Convert the roots from ClassId to NodeId using the choices.
        // std::vector<NodeId> node_roots;
        // for (const auto& cid : roots) {
        //     node_roots.push_back(choices.at(cid));
        // }
        std::unordered_map<NodeId, Cost> memo;
        return tree_cost_rec(egraph, roots, memo);
    }

    // ----------------------------------------------------------------
    // Compute the cost over the DAG of the extraction.
    // This may loop if there are cycles.
    // ----------------------------------------------------------------
    Cost dag_cost(const EGraph& egraph,
                  const std::vector<ClassId>& roots) const {
        // Using an ordered container is not essential.
        tsl::ordered_map<ClassId,Cost> costs;
        std::vector<ClassId> todo = roots; // copy
        while (!todo.empty()) {
            ClassId cid = todo.back();
            todo.pop_back();
            // Look up the selected node for this class.
            const NodeId& node_id = choices.at(cid);
            const Node& node = egraph[node_id];
            // If this class has already been processed, skip it.
            if (costs.find(cid) != costs.end()) {
                continue;
            }
            costs.insert({ cid, node.cost });
            for (const ClassId& child : node.children) {
                todo.push_back(child);
            }
        }
        // Sum all the costs.
        Cost total = 0;
        for (const auto& kv : costs) {
            total += kv.second;
        }
        return total;
    }

    // ----------------------------------------------------------------
    // Given a node and a mapping (that supports map_get) from ClassId to Cost,
    // return the sum of the node's cost plus the cost of its children.
    // If a child's cost is missing, COST_INFINITY is used.
    // ----------------------------------------------------------------
    // template <typename Map>
    // Cost node_sum_cost(const EGraph& egraph,
    //                    const Node& node,
    //                    const Map& costs) const {
    //     Cost sum = node.cost;
    //     for (const NodeId& n : node.children) {
    //         const ClassId& cid = egraph.nid_to_cid(n);
    //         const auto* cost_ptr = map_get(costs, cid);
    //         // If no cost is found, use COST_INFINITY.
    //         sum += cost_ptr ? *cost_ptr : COST_INFINITY;
    //     }
    //     return sum;
    // }

    template <typename Map>
    Cost node_sum_cost(const EGraph& egraph,
                    const Node& node,
                    const Map& costs) const {
        Cost sum = node.cost;
        for (const ClassId& cid : node.children) {
            const auto* cost_ptr = map_get(costs, cid);
            sum += cost_ptr ? *cost_ptr : COST_INFINITY;
        }
        return sum;
    }

private:
    // ----------------------------------------------------------------
    // Status used in cycle detection.
    // ----------------------------------------------------------------
    enum class Status { Doing, Done };

    // ----------------------------------------------------------------
    // Recursive helper for cycle detection (depth-first search).
    // ----------------------------------------------------------------
    void cycle_dfs(const EGraph& egraph,
                   const ClassId& class_id,
                   tsl::ordered_map<ClassId,enum Status>& status,
                   std::vector<ClassId>& cycles) const {
        auto it = status.find(class_id);
        if (it != status.end()) {
            if (it->second == Status::Doing) {
                // Found a cycle.
                cycles.push_back(class_id);
            }
            // If status is Done, nothing to do.
            return;
        }
        // Mark as in progress.
        status[class_id] = Status::Doing;
        // Get the chosen node for this class.
        const NodeId& node_id = choices.at(class_id);
        const Node& node = egraph[node_id];
        // Recurse on all children.
        for (const ClassId& child_cid : node.children) {
            // const ClassId& child_cid = egraph.nid_to_cid(child);
            cycle_dfs(egraph, child_cid, status, cycles);
        }
        // Mark as done.
        status[class_id] = Status::Done;
    }

    // ----------------------------------------------------------------
    // Recursive helper for computing tree cost.
    // ----------------------------------------------------------------
    Cost tree_cost_rec(const EGraph& egraph,
                       const std::vector<ClassId>& roots,
                       std::unordered_map<NodeId, Cost>& memo) const {
        Cost cost = 0;
        // std::vector<NodeId> node_roots;
        // for (const auto& cid : roots) {
        //     node_roots.push_back(choices.at(cid));
        // }
        for (const ClassId& root : roots) {
            NodeId root_node = choices.at(root);
            auto memo_it = memo.find(root_node);
            if (memo_it != memo.end()) {
                cost += memo_it->second;
                continue;
            }
            // For the given root, get the class id and then the chosen node.
            // const ClassId& cid = egraph.nid_to_cid(root);
            // const NodeId& chosen_node = choices.at(cid);
            const Node& node = egraph[root_node];
            // The cost is the node's cost plus the cost of its children.
            Cost inner = node.cost + tree_cost_rec(egraph, node.children, memo);
            memo.insert({ root_node, inner });
            cost += inner;
        }
        return cost;
    }
};

#endif  // EXTRACTION_HPP
