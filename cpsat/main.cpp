// CpsatSolveOnly.cpp
#include "egraph_serialize.hpp"
#include <cassert>
#include "ortools/sat/cp_model.h"
#include <iostream>
#include <unordered_map>
#include <vector>
#include <limits>
#include <string>
#include <fstream>
#include <sstream>
#include <chrono>
#include <filesystem>
#include <iomanip>

using namespace operations_research;
using namespace sat;

std::unordered_map<std::string, std::string> parseCommandLine(int argc, char* argv[]) {
  std::unordered_map<std::string, std::string> params;
  
  for (int i = 1; i < argc; i++) {
    std::string arg = argv[i];
    
    // 检查参数是否以--开头
    if (arg.substr(0, 2) == "--") {
      std::string key = arg.substr(2); // 去掉--前缀
      
      // 检查是否有下一个参数且不是以--开头的
      if (i + 1 < argc && argv[i + 1][0] != '-') {
        params[key] = argv[i + 1];
        i++; // 跳过下一个参数，因为已经处理了
      } else {
        // 对于没有值的参数，设置为空字符串
        params[key] = "";
      }
    }
  }
  
  return params;
}

// 用于生成格式化文件名的辅助函数
std::string generateSolutionFileName(int solution_number, double objective_value, double time_seconds) {
  std::stringstream ss;
  ss << "solution_" << std::setw(5) << std::setfill('0') << solution_number 
     << "_obj_" << std::fixed << std::setprecision(2) << objective_value 
     << "_time_" << std::fixed << std::setprecision(2) << time_seconds << ".sol";
  return ss.str();
}

int main(int argc, char* argv[]) {
  try {
    // 解析命令行参数
    auto params = parseCommandLine(argc, argv);
    
    // 检查必须的参数是否存在
    std::vector<std::string> requiredParams = {"egraph_json_file", "output_sol_file", "log_file"};
    bool missingParams = false;
    
    for (const auto& param : requiredParams) {
      if (params.find(param) == params.end()) {
        std::cerr << "Missing required parameter: --" << param << std::endl;
        missingParams = true;
      }
    }
    
    if (missingParams) {
      std::cerr << "Usage: " << argv[0] << " --egraph_json_file <file> --output_sol_file <file> --log_file <file>"
          << "[--zero_node_mst <file>] [--total_gurobi_mst <file>] [--time_limit <seconds>] [--solution_pool_dir <dir>]" 
          << std::endl;
      return 1;
    }
    
    // 获取参数值
    std::string egraph_json_file = params["egraph_json_file"];
    std::string output_sol_file = params["output_sol_file"];
    std::string log_file = params["log_file"];
    
    // 设置可选参数
    std::string zero_node_file = "";
    std::string warm_start_file = "";
    double time_limit = std::numeric_limits<double>::infinity();
    std::string solution_pool_dir = "";
    
    if (params.find("time_limit") != params.end()) {
      try {
        time_limit = std::stod(params["time_limit"]);
        std::cout << "设置时间限制为: " << time_limit << " 秒" << std::endl;
      } catch (const std::exception& ex) {
        std::cerr << "无效的时间限制值: " << params["time_limit"] << std::endl;
        return 1;
      }
    }
    
    if (params.find("solution_pool_dir") != params.end()) {
      solution_pool_dir = params["solution_pool_dir"];
      // 确保解池目录存在
      if (!solution_pool_dir.empty()) {
        std::filesystem::create_directories(solution_pool_dir);
        std::cout << "Solution pool directory: " << solution_pool_dir << std::endl;
      }
    }

    if (params.find("zero_node_mst") != params.end()) {
      zero_node_file = params["zero_node_mst"];
      std::cout << "零节点文件: " << zero_node_file << std::endl;
    }

    if (params.find("total_gurobi_mst") != params.end()) {
      warm_start_file = params["total_gurobi_mst"];
      std::cout << "热启动文件: " << warm_start_file << std::endl;
    }

    

    // 加载 EGraph 数据
    Data data = Data::from_json_file(egraph_json_file);
    EGraph egraph = EGraph::from_Data(data);
    std::vector<ClassId> roots = egraph.root_eclasses;
    const auto &classes = egraph.classes();
    
    // 定义模型中变量存放容器
    std::unordered_map<ClassId, BoolVar> active;
    std::unordered_map<ClassId, std::unordered_map<NodeId, BoolVar>> nodes_vars;

    // 创建 CP-SAT 模型构建器
    CpModelBuilder cp_model;
    for (const auto &entry : classes) {
      const ClassId &cid = entry.first;
      const Class &cls = entry.second;
      active[cid] = cp_model.NewBoolVar();
      std::unordered_map<NodeId, BoolVar> vars;
      for (size_t i = 0; i < cls.nodes.size(); ++i) {
        auto node = cls.nodes[i];
        auto node_id = egraph[node].id;
        vars.insert({node_id, cp_model.NewBoolVar()});
      }
      nodes_vars[cid] = vars;
      // 添加约束：当前 e-class 的激活状态等于其内部节点变量之和
      LinearExpr sum_nodes;
      for (const auto &v : nodes_vars[cid]) {
        sum_nodes += v.second;
      }
      cp_model.AddEquality(sum_nodes, active[cid]);
    }

    // 对于每个节点，如果其被选中，则要求所有其子 e-class 也必须被激活
    for (const auto &entry : classes) {
      const ClassId &cid = entry.first;
      const Class &cls = entry.second;
      const auto &vars = nodes_vars[cid];
      for (size_t i = 0; i < cls.nodes.size(); ++i) {
        NodeId node_id = cls.nodes[i];
        const Node &node = egraph[node_id];
        for (const auto &child_cid : node.children) {
          cp_model.AddLessOrEqual(vars.at(node_id), active[child_cid]);
        }
      }
    }

    // 目标函数：最小化所有选中节点的成本
    LinearExpr objective;
    for (const auto &entry : classes) {
      const ClassId &cid = entry.first;
      const Class &cls = entry.second;
      const auto &vars = nodes_vars[cid];
      for (size_t i = 0; i < cls.nodes.size(); ++i) {
        NodeId node_id = cls.nodes[i];
        const Node &node = egraph[node_id];
        int int_cost = static_cast<int>(node.cost);
        if (int_cost != 0) {
          if (int_cost == 1) {
            objective += vars.at(node_id);
          } else {
            objective += int_cost * vars.at(node_id);
          }
        }
      }
    }
    cp_model.Minimize(objective);

    // 强制根 e-class 激活
    for (const auto &root : roots) {
      cp_model.AddEquality(active[root], 1);
    }

    // 为每个 e-class 创建层次变量，用于防止环路（范围 [0, num_classes]）
    int num_classes = classes.size();
    std::unordered_map<ClassId, IntVar, std::hash<ClassId>> level;
    for (const auto &entry : classes) {
      const ClassId &cid = entry.first;
      level[cid] = cp_model.NewIntVar(Domain(0, num_classes));
    }

    // 添加约束：如果一个节点被选中，则其所有子 e-class 的层次至少比当前 e-class 高 1
    for (const auto &entry : classes) {
      const ClassId &cid = entry.first;
      const Class &cls = entry.second;
      const auto &vars = nodes_vars[cid];
      for (size_t i = 0; i < cls.nodes.size(); ++i) {
        NodeId node_id = cls.nodes[i];
        const Node &node = egraph[node_id];
        for (const auto &child_cid : node.children) {
          cp_model.AddGreaterOrEqual(LinearExpr(level[child_cid]) - level[cid], 1)
              .OnlyEnforceIf(vars.at(node_id));
        }
      }
    }

    // 读取 zero_node.mst 文件，将对应节点变量固定为 0
    if (!zero_node_file.empty()) {
      std::ifstream fin(zero_node_file);
      if (!fin.is_open()) {
        std::cerr << "无法打开文件: " << zero_node_file << std::endl;
        return 1;
      }
      std::string line;
      while (std::getline(fin, line)) {
        if (line.empty()) continue;
        // 期望格式: "N_<classid>_<node_index>"
        if (line.rfind("N_", 0) == 0) {
          size_t pos1 = line.find('_', 2);
          if (pos1 == std::string::npos) {
            std::cerr << "格式错误: " << line << std::endl;
            continue;
          }
          std::string classid_str = line.substr(2, pos1 - 2);
          std::string node_index_str = line.substr(pos1 + 1);
          try {
            unsigned int classid_val = std::stoul(classid_str);
            size_t node_index = std::stoul(node_index_str);
            ClassId cid(classid_val);
            NodeId nid(classid_val, node_index);
            auto it = nodes_vars.find(cid);
            if (it != nodes_vars.end()) {
              std::unordered_map<NodeId, BoolVar>& var_map = it->second;
              if (var_map.find(nid) != var_map.end()) {
                cp_model.AddEquality(var_map[nid], 0);
              } else {
                std::cerr << "节点索引 " << node_index << " 超出 ClassId " << classid_val << " 的范围." << std::endl;
              }
            } else {
              std::cerr << "未在 nodes_vars 中找到 ClassId " << classid_val << std::endl;
            }
          } catch (const std::exception& ex) {
            std::cerr << "解析错误, 行: " << line << ", 异常: " << ex.what() << std::endl;
          }
        } else {
          std::cerr << "忽略非预期格式的行: " << line << std::endl;
        }
      }
    }

    // 添加 warm start hint：解析 total_gurobi.mst 文件
    if (!warm_start_file.empty()) {
      std::ifstream warm_fin(warm_start_file);
      if (!warm_fin.is_open()) {
        std::cerr << "无法打开文件: " << warm_start_file << std::endl;
        return 1;
      }
      std::string line;
      while (std::getline(warm_fin, line)) {
        if (line.empty()) continue;
        std::istringstream iss(line);
        std::string token;
        int hint_value;
        if (!(iss >> token >> hint_value)) {
          std::cerr << "解析行失败: " << line << std::endl;
          continue;
        }
        if (token.rfind("N_", 0) == 0) {
          // token 格式: "N_<classid>_<node_index>"
          size_t pos1 = token.find('_', 2);
          if (pos1 == std::string::npos) {
            std::cerr << "格式错误: " << token << std::endl;
            continue;
          }
          std::string classid_str = token.substr(2, pos1 - 2);
          std::string node_index_str = token.substr(pos1 + 1);
          try {
            unsigned int classid_val = std::stoul(classid_str);
            size_t node_index = std::stoul(node_index_str);
            ClassId cid(classid_val);
            NodeId nid(classid_val, node_index);
            auto it = nodes_vars.find(cid);
            if (it != nodes_vars.end()) {
              std::unordered_map<NodeId, BoolVar>& var_map = it->second;
              auto var_it = var_map.find(nid);
              if (var_it != var_map.end()) {
                cp_model.AddHint({var_it->second}, {hint_value});
              } else {
                std::cerr << "未找到节点 " << token << " 在 ClassId " << classid_val << std::endl;
              }
            } else {
              std::cerr << "未在 nodes_vars 中找到 ClassId " << classid_val << std::endl;
            }
          } catch (const std::exception &ex) {
            std::cerr << "解析错误, token: " << token << ", 异常: " << ex.what() << std::endl;
          }
        } else if (token.rfind("A_", 0) == 0) {
          // token 格式: "A_<classid>"
          std::string classid_str = token.substr(2);
          try {
            unsigned int classid_val = std::stoul(classid_str);
            ClassId cid(classid_val);
            auto it = active.find(cid);
            if (it != active.end()) {
              cp_model.AddHint({it->second}, {hint_value});
            } else {
              std::cerr << "未找到激活变量 A_" << classid_val << std::endl;
            }
          } catch (const std::exception &ex) {
            std::cerr << "解析错误, token: " << token << ", 异常: " << ex.what() << std::endl;
          }
        } else {
          std::cerr << "忽略未知格式的行: " << token << std::endl;
        }
      }
    }

    // 设置求解器观察器、参数并求解
    Model model;
    int num_solutions = 0;
    auto start = std::chrono::high_resolution_clock::now();
    std::string log = "";
    model.Add(NewFeasibleSolutionObserver([&](const CpSolverResponse& r) {
        auto current_time = std::chrono::high_resolution_clock::now();
        double elapsed_seconds = std::chrono::duration_cast<std::chrono::duration<double>>(
                        current_time - start).count();
        std::cout << "Incumbent Solution: " 
                        << elapsed_seconds 
                        << "s; objective: " << r.objective_value() << std::endl;
        log += std::to_string(elapsed_seconds) + " " + std::to_string(r.objective_value()) + "\n";

        // 如果指定了solution_pool目录，则保存当前解
        if (!solution_pool_dir.empty()) {
          // 生成解输出内容
          std::string output = "";
          for (const auto& entry : nodes_vars) {
              const ClassId& cid = entry.first;
              const auto& node_map = entry.second;
              for (const auto& node_entry : node_map) {
                  const NodeId& node_id = node_entry.first;
                  const BoolVar& var = node_entry.second;
                  bool value = SolutionBooleanValue(r, var);
                  assert(cid.id == node_id.id[0]);
                  output += "N_" + std::to_string(cid.id) + "_" + std::to_string(node_id.id[1]) + " " + std::to_string(value) + "\n";
              }
          }
          
          // 生成唯一的文件名
          std::string solution_filename = generateSolutionFileName(
              num_solutions, r.objective_value(), elapsed_seconds);
          std::string full_path = solution_pool_dir + "/" + solution_filename;
          
          // 保存到文件
          std::ofstream sol_file(full_path);
          if (sol_file.is_open()) {
              sol_file << output;
              sol_file.close();
              std::cout << "Saved incumbent solution to: " << full_path << std::endl;
          } else {
              std::cerr << "无法创建解文件: " << full_path << std::endl;
          }
        }
      
        num_solutions++;
    }));

    if (time_limit != std::numeric_limits<double>::infinity()) {
      SatParameters parameters;
      parameters.set_max_time_in_seconds(time_limit);
      // parameters.set_max_time_in_seconds(3600.0);
      model.Add(NewSatParameters(parameters));
    }

    const CpSolverResponse response = SolveCpModel(cp_model.Build(), &model);

    if (response.status() == CpSolverStatus::OPTIMAL || response.status() == CpSolverStatus::FEASIBLE) {
        std::string output = "";
        for (const auto& entry : nodes_vars) {
            const ClassId& cid = entry.first;
            const auto& node_map = entry.second;
            for (const auto& node_entry : node_map) {
              const NodeId& node_id = node_entry.first;
              const BoolVar& var = node_entry.second;
              bool value = SolutionBooleanValue(response, var);
              assert(cid.id == node_id.id[0]);
              // std::cout << "N_" << cid.id << "_" << node_id.id[1] << ": " << value << std::endl;
              output += "N_" + std::to_string(cid.id) + "_" + std::to_string(node_id.id[1]) + " " + std::to_string(value) + "\n";
            }
        }
        // Save the output to the specified output file
        std::cout << "Saving output to " << output_sol_file << std::endl;
        std::ofstream fout(output_sol_file);
        if (!fout.is_open()) {
          std::cerr << "Unable to open output file: " << output_sol_file << std::endl;
          return 1;
        }
        fout << output;
        fout.close();
        // Save the log to the specified log file
        std::cout << "Saving log to " << log_file << std::endl;
        std::ofstream log_fout(log_file);
        if (!log_fout.is_open()) {
          std::cerr << "Unable to open log file: " << log_file << std::endl;
          return 1;
        }
        log_fout << log;
        log_fout.close();
        std::cout << "Output saved." << std::endl;
        std::cout << "Objective: " << response.objective_value() << std::endl;
        std::cout << "Runtime:" << response.wall_time() << " s" << std::endl;
        std::cout << "Number of solutions found: " << num_solutions << std::endl;
    } else {
        std::cout << "No solution found." << std::endl;
    }

  } catch (const std::exception &ex) {
    std::cerr << "Error: " << ex.what() << std::endl;
    return 1;
  }
  return 0;
}
