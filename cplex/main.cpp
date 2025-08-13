#include <ilcplex/ilocplex.h>
#include <ilconcert/ilomodel.h>
#include <iostream>
#include <fstream>
#include <sstream>
#include <unordered_map>
#include <chrono>
#include <iomanip>
#include <filesystem>
#include <vector>
#include <cmath>

ILOSTLBEGIN

// Helper function: generate solution file name
std::string generateSolutionFileName(int solution_number, double objective_value, double time_seconds) {
    std::stringstream ss;
    ss << "solution_" << std::setw(5) << std::setfill('0') << solution_number 
       << "_obj_" << std::fixed << std::setprecision(2) << objective_value 
       << "_time_" << std::fixed << std::setprecision(2) << time_seconds << ".sol";
    return ss.str();
}

// Parse command line arguments
std::unordered_map<std::string, std::string> parseCommandLine(int argc, char* argv[]) {
    std::unordered_map<std::string, std::string> params;
    for (int i = 1; i < argc; i++) {
        std::string arg = argv[i];
        if (arg.substr(0, 2) == "--") {
            std::string key = arg.substr(2);
            if (i + 1 < argc && argv[i+1][0] != '-') {
                params[key] = argv[i+1];
                i++;
            } else {
                params[key] = "";
            }
        }
    }
    return params;
}

// CPLEX callback: capture each new incumbent solution, log only improved solutions, and save current solution when solution_pool_dir is not empty
class MyIncumbentCallback : public IloCplex::IncumbentCallbackI {
public:
    MyIncumbentCallback(IloEnv env,
                        const std::string& poolDir,
                        std::ofstream& logStream,
                        int& incCount,
                        std::chrono::time_point<std::chrono::high_resolution_clock> start,
                        IloNumVarArray _vars)
    : IloCplex::IncumbentCallbackI(env),
      solutionPoolDir(poolDir),
      log(logStream),
      incumbentCount(incCount),
      startTime(start),
      vars(_vars)
    {
        bestObj = std::numeric_limits<double>::infinity();
    }

    void main() override {
        double obj = getObjValue();
        auto current_time = std::chrono::high_resolution_clock::now();
        double elapsed = std::chrono::duration<double>(current_time - startTime).count();
        // 记录日志：只有当目标改进时写入 log
        if (obj < bestObj) {
            bestObj = obj;
            log << elapsed << ": " << obj << std::endl;
        }
        // 如果提供了 pool 目录，则保存当前解
        if (!solutionPoolDir.empty()) {
            std::stringstream ss;
            ss << solutionPoolDir << "/" << generateSolutionFileName(incumbentCount, obj, elapsed);
            std::string fullPath = ss.str();

            std::ofstream solFile(fullPath);
            if (solFile.is_open()) {
                for (IloInt i = 0; i < vars.getSize(); i++) {
                    double val = getValue(vars[i]);
                    if (fabs(val - 1.0) < 1e-5) {
                        solFile << vars[i].getName() << " " << 1 << std::endl;
                    }
                }
                solFile.close();
                std::cout << "Saved incumbent solution #" << incumbentCount+1 
                          << " at " << elapsed << " seconds, objective: " << obj 
                          << " to: " << fullPath << std::endl;
            } else {
                std::cerr << "Failed to create solution file: " << fullPath << std::endl;
            }
        }
        incumbentCount++;
    }

    IloCplex::CallbackI* duplicateCallback() const override {
        return (new (getEnv()) MyIncumbentCallback(getEnv(), solutionPoolDir, log, incumbentCount, startTime, vars));
    }

private:
    std::string solutionPoolDir;
    std::ofstream& log;
    int& incumbentCount;
    double bestObj;
    std::chrono::time_point<std::chrono::high_resolution_clock> startTime;
    IloNumVarArray vars;
};

void writePlainSolution(IloCplex& cplex, IloNumVarArray& vars, const std::string& fileName) {
    std::ofstream out(fileName);
    if (!out.is_open()) {
        std::cerr << "Could not open file " << fileName << " for writing solution." << std::endl;
        return;
    }
    for (IloInt i = 0; i < vars.getSize(); i++){
        double val = cplex.getValue(vars[i]);
        out << vars[i].getName() << " " << val << std::endl;
    }
    out.close();
}

int main(int argc, char* argv[]) {
    try {
        // 解析命令行参数
        auto params = parseCommandLine(argc, argv);
        std::vector<std::string> requiredParams = {"lp_file", "output_file", "log_file"};
        bool missingParams = false;
        for (const auto& key : requiredParams) {
            if (params.find(key) == params.end()) {
                std::cerr << "Missing required parameter: --" << key << std::endl;
                missingParams = true;
            }
        }
        if (missingParams) {
            std::cerr << "Usage: " << argv[0] 
                      << " --lp_file <file> --output_file <file> --log_file <file> "
                      << "[--mst_file <file>] [--time_limit <seconds>] [--solution_pool_dir <dir>]" 
                      << std::endl;
            return 1;
        }

        std::string lp_file = params["lp_file"];
        std::string output_file = params["output_file"];
        std::string log_file = params["log_file"];
        std::string mst_file = "";
        double time_limit = 1e+20;  // 默认无限制
        std::string solution_pool_dir = "";
        
        if (params.find("mst_file") != params.end()) {
            mst_file = params["mst_file"];
            if (!std::filesystem::exists(mst_file)) {
                std::cerr << "Warning: MST file does not exist: " << mst_file << std::endl;
                mst_file = "";
            }
        }
        
        if (params.find("time_limit") != params.end()) {
            try {
                time_limit = std::stod(params["time_limit"]);
                std::cout << "Setting time limit to: " << time_limit << " seconds" << std::endl;
            } catch (...) {
                std::cerr << "Invalid time limit value: " << params["time_limit"] << std::endl;
                return 1;
            }
        }
        
        if (params.find("solution_pool_dir") != params.end()) {
            solution_pool_dir = params["solution_pool_dir"];
            if (!solution_pool_dir.empty()) {
                std::filesystem::create_directories(solution_pool_dir);
                std::cout << "Solution pool directory: " << solution_pool_dir << std::endl;
            }
        }
        
        // 打开 log 文件
        std::ofstream logStream(log_file);
        if (!logStream.is_open()) {
            std::cerr << "Failed to open log file: " << log_file << std::endl;
            return 1;
        }
        
        IloEnv env;
        IloModel model(env);
        IloCplex cplex(env);
        
        if (!std::filesystem::exists(lp_file)) {
            std::cerr << "LP file not found: " << lp_file << std::endl;
            return 1;
        }
        cplex.importModel(model, lp_file.c_str());
        cplex.extract(model);
        
        // 设置时间限制和线程参数
        cplex.setParam(IloCplex::Param::TimeLimit, time_limit);
        cplex.setParam(IloCplex::Param::Threads, 0);
        
        // 获取模型中的所有变量（使用模型迭代器）
        IloNumVarArray vars(env);
        std::unordered_map<std::string, IloNumVar> varMap;
        IloModel::Iterator it(model);
        while (it.ok()) {
            IloExtractable ext = *it;
            if (ext.isVariable()) {
                vars.add(ext.asVariable());
                IloNumVar var = ext.asVariable();
                varMap[std::string(var.getName())] = var;
            }
            ++it;
        }


        int incumbentCount = 0;
        auto startTime = std::chrono::high_resolution_clock::now();
        
        // 始终注册回调，这样无论是否提供 pool 参数，日志都能打印
        cplex.use(new (env) MyIncumbentCallback(env, solution_pool_dir, logStream, incumbentCount, startTime, vars));
        
        // if (!mst_file.empty()) {
        //     std::cout << "Loading initial solution (MIP start) from: " << mst_file << std::endl;
        //     std::ifstream infile(mst_file);
        //     if (!infile.is_open()) {
        //         std::cerr << "Cannot open MIP start file: " << mst_file << std::endl;
        //     } else {
        //         // 用于存放 warm start 的变量和对应的初始值
        //         IloNumVarArray startVars(env);
        //         IloNumArray startVals(env);
        //         std::string varName;
        //         double val;
        //         // 每行格式：变量名 初始值
        //         while (infile >> varName >> val) {
        //             std::cout << varName << "  " << val << std::endl;
        //             // 遍历模型中的变量，找到名称匹配的变量
        //             for (IloInt i = 0; i < vars.getSize(); i++) {
        //                 if (std::string(vars[i].getName()) == varName) {
        //                     startVars.add(vars[i]);
        //                     startVals.add(val);
        //                     break;
        //                 }
        //             }
        //         }
        //         infile.close();
        //         // 如果读取到至少一个变量，就将其作为 MIP start 添加到求解器中
        //         if (startVars.getSize() > 0) {
        //             cplex.addMIPStart(startVars, startVals, IloCplex::MIPStartAuto);
        //             std::cout << "MIP start loaded successfully with " << startVars.getSize() << " variables." << std::endl;
        //         } else {
        //             std::cout << "No valid MIP start variables found in file." << std::endl;
        //         }
        //         // 注意：如果不需要再使用 startVars/startVals，可以释放资源
        //         startVals.end();
        //         startVars.end();
        //     }
        //     std::cout << "Loading Finished" << std::endl;
        // }

        std::ifstream infile(mst_file);
        if (!infile.is_open()) {
            std::cerr << "Cannot open MIP start file: " << mst_file << std::endl;
        } else {
            IloNumVarArray startVars(env);
            IloNumArray startVals(env);
            std::string varName;
            double val;
            // 每行格式：变量名 初始值
            while (infile >> varName >> val) {
                auto it = varMap.find(varName);
                if (it != varMap.end()) {
                    startVars.add(it->second);
                    startVals.add(val);
                }
            }
            infile.close();
        
            if (startVars.getSize() > 0) {
                cplex.addMIPStart(startVars, startVals, IloCplex::MIPStartAuto);
                std::cout << "MIP start loaded successfully with " << startVars.getSize() << " variables." << std::endl;
            } else {
                std::cout << "No valid MIP start variables found in file." << mst_file << std::endl;
            }
            startVals.end();
            startVars.end();
        }
        
        
        
        std::cout << "Starting optimization..." << std::endl;
        bool solved = cplex.solve();
        double totalRuntime = cplex.getCplexTime();
        
        // 状态判断
        IloAlgorithm::Status status = cplex.getStatus();
        if (status == IloAlgorithm::Optimal) {
            std::cout << "Optimal solution found!" << std::endl;
        } else if (status == IloAlgorithm::Infeasible) {
            std::cout << "Model is infeasible." << std::endl;
        } else if (status == IloAlgorithm::Unbounded) {
            std::cout << "Model is unbounded." << std::endl;
        } else {
            std::cout << "Optimization ended with status: " << status << std::endl;
        }
        
        if (solved) {
            double obj_val = cplex.getObjValue();
            std::cout << "Objective value: " << obj_val << std::endl;
            writePlainSolution(cplex, vars, output_file);
            std::cout << "Solution saved to: " << output_file << std::endl;
            
            // 如果有 solution_pool_dir，则保存最终解
            if (!solution_pool_dir.empty()) {
                std::stringstream ss;
                auto current_time = std::chrono::high_resolution_clock::now();
                double elapsed = std::chrono::duration<double>(current_time - startTime).count();
                ss << solution_pool_dir << "/" << generateSolutionFileName(incumbentCount, obj_val, elapsed);
                std::string fullPath = ss.str();
                writePlainSolution(cplex, vars, fullPath);
            }
        } else {
            std::cout << "No solution found." << std::endl;
        }
        
        logStream.close();
        env.end();
    }
    catch (IloException& e) {
        std::cerr << "Concert exception caught: " << e.getMessage() << std::endl;
        return 1;
    }
    catch (std::exception &e) {
        std::cerr << "Standard exception caught: " << e.what() << std::endl;
        return 1;
    }
    
    return 0;
}
