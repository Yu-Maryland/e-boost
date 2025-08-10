#ifndef EGRAPH_HPP
#define EGRAPH_HPP

#include <array>
#include <cstdint>
#include <map>
#include <optional>
#include <sstream>
#include <stdexcept>
#include <string>
#include <tuple>
#include <vector>
#include <fstream>
#include <iostream>
#include <cctype>
#include <functional>

// ------------------------
// 类型别名
// ------------------------
using Cost = double; // 对应 Rust 中 NotNan<f64>

// ------------------------
// 基本类型定义
// ------------------------

// NodeId：使用两个 uint32_t 表示
struct NodeId {
    std::array<uint32_t, 2> id;

    NodeId() = default;
    NodeId(uint32_t a, uint32_t b) : id{{a, b}} {}

    bool operator<(const NodeId &other) const {
        return std::tie(id[0], id[1]) < std::tie(other.id[0], other.id[1]);
    }
    bool operator==(const NodeId &other) const {
        return id[0] == other.id[0] && id[1] == other.id[1];
    }
};

inline std::ostream& operator<<(std::ostream &os, const NodeId &nid) {
    os << nid.id[0] << '.' << nid.id[1];
    return os;
}

// NodeId_old：使用字符串存储，格式如 "a.b"
struct NodeId_old {
    std::string id;

    NodeId_old() = default;
    explicit NodeId_old(const std::string &s) : id(s) {}

    bool operator<(const NodeId_old &other) const {
        return id < other.id;
    }
    bool operator==(const NodeId_old &other) const {
        return id == other.id;
    }
};

// ClassId：单个 uint32_t 的包装
struct ClassId {
    uint32_t id;

    ClassId() = default;
    explicit ClassId(uint32_t i) : id(i) {}

    bool operator<(const ClassId &other) const {
        return id < other.id;
    }
    bool operator==(const ClassId &other) const {
        return id == other.id;
    }
};

inline std::ostream& operator<<(std::ostream &os, const ClassId &cid) {
    os << cid.id;
    return os;
}

// ------------------------
// 节点及数据结构（新版与旧版）
// ------------------------

// 旧版本节点（用于数据转换）
struct Node_old {
    std::string op;
    NodeId_old id;
    std::vector<ClassId> children;
    ClassId eclass;
    Cost cost = 1.0;
};

// 节点（新版）
struct Node {
    std::string op;
    NodeId id;
    std::vector<ClassId> children;
    ClassId eclass;
    Cost cost = 1.0;

    bool is_leaf() const {
        return children.empty();
    }
};

// ------------------------
// Data 结构定义
// ------------------------

// 旧版 Data，用于兼容
struct Data_old {
    std::map<NodeId_old, Node_old> nodes;
    std::vector<ClassId> root_eclasses;

    // 此处不实现 JSON 解析，按需扩展
};

// 新版 Data 结构
struct Data {
    std::map<NodeId, Node> nodes;
    std::vector<ClassId> root_eclasses;

    // 从 JSON 文件读取 Data
    static Data from_json_file(const std::string &path);
    // 将 Data 写入 JSON 文件（示例实现）
    void to_json_file(const std::string &path) const;
};

// ------------------------
// 转换函数
// ------------------------

// 将旧版 NodeId_old 转换为新版 NodeId（假定格式为 "a.b"）
inline NodeId convert_nodeid_old(const NodeId_old &old) {
    size_t dotPos = old.id.find('.');
    if (dotPos == std::string::npos) {
        throw std::runtime_error("Invalid NodeId_old format: " + old.id);
    }
    uint32_t a = std::stoul(old.id.substr(0, dotPos));
    uint32_t b = std::stoul(old.id.substr(dotPos + 1));
    return NodeId(a, b);
}

// 将新版 NodeId 转换为旧版 NodeId_old（格式为 "a.b"）
inline NodeId_old convert_nodeid_to_old(const NodeId &node_id) {
    std::ostringstream oss;
    oss << node_id.id[0] << '.' << node_id.id[1];
    return NodeId_old(oss.str());
}

// 根据旧版 Data_old 构造新版 Data
inline Data Data_from_Data_old(const Data_old &data_old) {
    Data data;
    for (const auto &pair : data_old.nodes) {
        const NodeId_old &old_id = pair.first;
        const Node_old &old_node = pair.second;
        NodeId new_id = convert_nodeid_old(old_id);
        Node new_node;
        new_node.op = old_node.op;
        new_node.id = new_id;
        new_node.children = old_node.children;
        new_node.eclass = old_node.eclass;
        new_node.cost = old_node.cost;
        data.nodes.insert({new_id, new_node});
    }
    data.root_eclasses = data_old.root_eclasses;
    return data;
}

// 将新版 Data 转换为旧版 Data_old（节点 id 格式转换）
inline Data_old Data_to_Data_old(const Data &data) {
    Data_old data_old;
    for (const auto &pair : data.nodes) {
        const NodeId &node_id = pair.first;
        const Node &node = pair.second;
        NodeId_old old_id = convert_nodeid_to_old(node_id);
        Node_old node_old;
        node_old.op = node.op;
        node_old.id = old_id;
        node_old.children = node.children;
        node_old.eclass = node.eclass;
        node_old.cost = node.cost;
        data_old.nodes.insert({old_id, node_old});
    }
    data_old.root_eclasses = data.root_eclasses;
    return data_old;
}

// ------------------------
// EGraph 数据结构定义
// ------------------------

// e-class，包含一个 id 和该类中的所有节点
struct Class {
    ClassId id;
    std::vector<NodeId> nodes;
};

// 附加数据（例如类型信息）与 e-class 关联
struct ClassData {
    std::optional<std::string> typ;
};

class EGraph {
public:
    // 节点映射：使用 std::map 模拟 Rust 中的 IndexMap（按 key 排序）
    std::map<NodeId, Node> nodes;
    std::vector<ClassId> root_eclasses;
    std::map<ClassId, ClassData> class_data;

private:
    // 一次性计算缓存
    mutable std::optional<std::map<ClassId, Class>> once_cell_classes;

public:
    EGraph() = default;

    // 添加新节点，若节点已存在则抛出异常
    void add_node(const NodeId &node_id, const Node &node) {
        auto [it, inserted] = nodes.insert({node_id, node});
        if (!inserted) {
            std::ostringstream oss;
            oss << "Duplicate node with id " << node_id << "\n"
                << "old: " << it->second.op << "\n"
                << "new: " << node.op;
            throw std::runtime_error(oss.str());
        }
    }

    // 根据节点 id 返回对应的 e-class id
    const ClassId& nid_to_cid(const NodeId &node_id) const {
        auto it = nodes.find(node_id);
        if (it == nodes.end()) {
            std::ostringstream oss;
            oss << "No node with id " << node_id;
            throw std::runtime_error(oss.str());
        }
        return it->second.eclass;
    }

    // 根据节点 id 返回对应的 e-class（依赖 classes() 计算结果）
    const Class& nid_to_class(const NodeId &node_id) const {
        const ClassId &cid = nid_to_cid(node_id);
        const auto &cls_map = classes();
        auto it = cls_map.find(cid);
        if (it == cls_map.end()) {
            std::ostringstream oss;
            oss << "No class with id " << cid;
            throw std::runtime_error(oss.str());
        }
        return it->second;
    }

    // 按 e-class 将所有节点分组，并缓存结果（仅第一次调用时计算）
    const std::map<ClassId, Class>& classes() const {
        if (!once_cell_classes.has_value()) {
            std::map<ClassId, Class> classes;
            for (const auto &pair : nodes) {
                const NodeId &node_id = pair.first;
                const Node &node = pair.second;
                auto &cls = classes[node.eclass];
                if (cls.nodes.empty()) {
                    cls.id = node.eclass;
                }
                cls.nodes.push_back(node_id);
            }
            once_cell_classes = std::move(classes);
        }
        return *once_cell_classes;
    }

    // 重载下标操作符，根据 NodeId 返回节点
    const Node& operator[](const NodeId &node_id) const {
        auto it = nodes.find(node_id);
        if (it == nodes.end()) {
            std::ostringstream oss;
            oss << "No node with id " << node_id;
            throw std::runtime_error(oss.str());
        }
        return it->second;
    }

    // 根据 ClassId 返回 e-class
    const Class& operator[](const ClassId &class_id) const {
        const auto &cls_map = classes();
        auto it = cls_map.find(class_id);
        if (it == cls_map.end()) {
            std::ostringstream oss;
            oss << "No class with id " << class_id;
            throw std::runtime_error(oss.str());
        }
        return it->second;
    }

    // 从 Data 构造 EGraph
    static EGraph from_Data(const Data &data) {
        EGraph egraph;
        egraph.nodes = data.nodes;
        egraph.root_eclasses = data.root_eclasses;
        egraph.once_cell_classes.reset();
        return egraph;
    }

    // 直接从 JSON 文件构造 EGraph（内部调用 Data::from_json_file）
    static EGraph from_json_file(const std::string &path) {
        Data data = Data::from_json_file(path);
        return from_Data(data);
    }
};

// ------------------------
// JSON 解析相关辅助函数（仅支持固定格式）
// ------------------------
namespace {
    // 跳过空白字符
    inline void skipWhitespace(const std::string &s, size_t &i) {
        while (i < s.size() && std::isspace(s[i])) {
            ++i;
        }
    }

    // 解析一个双引号括起来的字符串（不处理转义，仅支持简单情况）
    inline std::string parseString(const std::string &s, size_t &i) {
        skipWhitespace(s, i);
        if (s[i] != '"') {
            throw std::runtime_error("Expected '\"' at position " + std::to_string(i));
        }
        ++i; // 跳过起始 "
        std::string result;
        while (i < s.size() && s[i] != '"') {
            result.push_back(s[i]);
            ++i;
        }
        if (i >= s.size() || s[i] != '"') {
            throw std::runtime_error("Expected closing '\"' in string");
        }
        ++i; // 跳过结束 "
        return result;
    }

    // 解析数字（支持整数和浮点数）
    inline double parseNumber(const std::string &s, size_t &i) {
        skipWhitespace(s, i);
        size_t start = i;
        while (i < s.size() && (std::isdigit(s[i]) || s[i] == '.' || s[i]=='-' || s[i]=='+' )) {
            ++i;
        }
        std::string numStr = s.substr(start, i - start);
        return std::stod(numStr);
    }

    // 解析整数
    inline int parseInt(const std::string &s, size_t &i) {
        skipWhitespace(s, i);
        size_t start = i;
        while (i < s.size() && (std::isdigit(s[i]) || s[i]=='-' || s[i]=='+' )) {
            ++i;
        }
        std::string intStr = s.substr(start, i - start);
        return std::stoi(intStr);
    }

    // 解析一个整数数组，格式如 [0, 1, 2]
    inline std::vector<ClassId> parseIntArray(const std::string &s, size_t &i) {
        skipWhitespace(s, i);
        if (s[i] != '[') {
            throw std::runtime_error("Expected '[' at position " + std::to_string(i));
        }
        ++i; // 跳过 '['
        std::vector<ClassId> arr;
        skipWhitespace(s, i);
        if (s[i] == ']') { // 空数组
            ++i;
            return arr;
        }
        while (true) {
            skipWhitespace(s, i);
            int num = parseInt(s, i);
            arr.push_back(ClassId(static_cast<uint32_t>(num)));
            skipWhitespace(s, i);
            if (s[i] == ',') {
                ++i; // 跳过逗号
            } else if (s[i] == ']') {
                ++i; // 跳过 ]
                break;
            } else {
                throw std::runtime_error("Expected ',' or ']' in array at position " + std::to_string(i));
            }
        }
        return arr;
    }

    // 解析一个 Node 对象，假定 s[i] 指向 '{'
    inline Node parseNodeObject(const std::string &s, size_t &i) {
        skipWhitespace(s, i);
        if (s[i] != '{') {
            throw std::runtime_error("Expected '{' at beginning of node object");
        }
        ++i; // 跳过 '{'
        Node node;
        bool first = true;
        while (true) {
            skipWhitespace(s, i);
            if (s[i] == '}') {
                ++i; // 跳过 '}'
                break;
            }
            if (!first) {
                if (s[i] == ',') {
                    ++i;
                    skipWhitespace(s, i);
                } else {
                    throw std::runtime_error("Expected ',' between members in node object");
                }
            }
            first = false;
            std::string key = parseString(s, i);
            skipWhitespace(s, i);
            if (s[i] != ':') {
                throw std::runtime_error("Expected ':' after key in node object");
            }
            ++i; // 跳过 ':'
            skipWhitespace(s, i);
            if (key == "op") {
                node.op = parseString(s, i);
            } else if (key == "cost") {
                node.cost = parseNumber(s, i);
            } else if (key == "eclass") {
                int eclass = parseInt(s, i);
                node.eclass = ClassId(static_cast<uint32_t>(eclass));
            } else if (key == "children") {
                node.children = parseIntArray(s, i);
            } else if (key == "id") {
                std::string idStr = parseString(s, i);
                size_t dotPos = idStr.find('.');
                if (dotPos == std::string::npos) {
                    throw std::runtime_error("Invalid id format: " + idStr);
                }
                uint32_t a = std::stoul(idStr.substr(0, dotPos));
                uint32_t b = std::stoul(idStr.substr(dotPos + 1));
                node.id = NodeId(a, b);
            } else {
                // 跳过未知键对应的值（支持字符串、数字、数组、对象）
                if (s[i] == '"') {
                    parseString(s, i);
                } else if (std::isdigit(s[i]) || s[i]=='-' || s[i]=='+' ) {
                    parseNumber(s, i);
                } else if (s[i] == '[') {
                    parseIntArray(s, i);
                } else if (s[i] == '{') {
                    int braceCount = 1;
                    ++i;
                    while (i < s.size() && braceCount > 0) {
                        if (s[i] == '{') ++braceCount;
                        else if (s[i] == '}') --braceCount;
                        ++i;
                    }
                } else {
                    throw std::runtime_error("Unexpected value in node object");
                }
            }
            skipWhitespace(s, i);
        }
        return node;
    }
} // end 匿名命名空间

// ------------------------
// Data::from_json_file 与 Data::to_json_file 实现
// ------------------------
inline Data Data::from_json_file(const std::string &path) {
    std::ifstream file(path);
    if (!file) {
        throw std::runtime_error("Cannot open file: " + path);
    }
    std::stringstream buffer;
    buffer << file.rdbuf();
    std::string content = buffer.str();
    size_t i = 0;
    skipWhitespace(content, i);
    if (content[i] != '{') {
        throw std::runtime_error("Expected '{' at beginning of JSON");
    }
    ++i; // 跳过 '{'
    Data data;
    bool firstTop = true;
    while (true) {
        skipWhitespace(content, i);
        if (content[i] == '}') {
            ++i;
            break;
        }
        if (!firstTop) {
            if (content[i] == ',') {
                ++i;
                skipWhitespace(content, i);
            } else {
                throw std::runtime_error("Expected ',' between top-level members");
            }
        }
        firstTop = false;
        std::string topKey = parseString(content, i);
        skipWhitespace(content, i);
        if (content[i] != ':') {
            throw std::runtime_error("Expected ':' after top-level key");
        }
        ++i; // 跳过 ':'
        skipWhitespace(content, i);
        if (topKey == "nodes") {
            if (content[i] != '{') {
                throw std::runtime_error("Expected '{' for nodes object");
            }
            ++i; // 跳过 '{'
            bool firstNode = true;
            while (true) {
                skipWhitespace(content, i);
                if (content[i] == '}') {
                    ++i;
                    break;
                }
                if (!firstNode) {
                    if (content[i] == ',') {
                        ++i;
                        skipWhitespace(content, i);
                    } else {
                        throw std::runtime_error("Expected ',' between node entries");
                    }
                }
                firstNode = false;
                // 节点 key，如 "0.0"
                std::string nodeKey = parseString(content, i);
                skipWhitespace(content, i);
                if (content[i] != ':') {
                    throw std::runtime_error("Expected ':' after node key");
                }
                ++i; // 跳过 ':'
                skipWhitespace(content, i);
                // 解析节点对象
                Node node = parseNodeObject(content, i);
                // 要求节点 key 与节点内部 id 格式一致（"a.b"）
                size_t dotPos = nodeKey.find('.');
                if (dotPos == std::string::npos) {
                    throw std::runtime_error("Invalid node key format: " + nodeKey);
                }
                uint32_t a = std::stoul(nodeKey.substr(0, dotPos));
                uint32_t b = std::stoul(nodeKey.substr(dotPos + 1));
                NodeId nid(a, b);
                data.nodes.insert({nid, node});
            }
        } else if (topKey == "root_eclasses") {
            if (content[i] != '[') {
                throw std::runtime_error("Expected '[' for root_eclasses array");
            }
            data.root_eclasses = parseIntArray(content, i);
        } else if (topKey == "class_data") {
            // 本示例中不处理 class_data，仅跳过
            if (content[i] != '{') {
                throw std::runtime_error("Expected '{' for class_data object");
            }
            int braceCount = 1;
            ++i;
            while (i < content.size() && braceCount > 0) {
                if (content[i] == '{') ++braceCount;
                else if (content[i] == '}') --braceCount;
                ++i;
            }
        } else {
            // 跳过未知顶层键对应的值
            while (i < content.size() && content[i] != ',' && content[i] != '}') {
                ++i;
            }
        }
        skipWhitespace(content, i);
    }
    return data;
}

inline void Data::to_json_file(const std::string &path) const {
    std::ofstream file(path);
    if (!file) {
        throw std::runtime_error("Cannot open file for writing: " + path);
    }
    // 此处仅输出一个简单的 JSON 结构示例
    file << "{\n  \"nodes\": {},\n  \"root_eclasses\": []\n}\n";
}


namespace std {
    template <>
    struct hash<ClassId> {
        std::size_t operator()(const ClassId &cid) const noexcept {
            return std::hash<uint32_t>{}(cid.id);
        }
    };

    template <>
    struct hash<NodeId> {
        std::size_t operator()(const NodeId &nid) const noexcept {
            std::size_t h1 = std::hash<uint32_t>{}(nid.id[0]);
            std::size_t h2 = std::hash<uint32_t>{}(nid.id[1]);
            // 使用 boost::hash_combine 的思路组合两个 hash 值
            return h1 ^ (h2 + 0x9e3779b97f4a7c15ULL + (h1 << 6) + (h1 >> 2));
        }
    };
}

#endif // EGRAPH_HPP
