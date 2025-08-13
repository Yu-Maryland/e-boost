#include <iostream>
#include <string>
#include <unordered_map>
#include <vector>
#include <filesystem>
#include <fstream>
#include <iomanip>
#include <chrono>
#include <cmath>
#include "gurobi_c++.h"

// Helper function to generate formatted solution filenames
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
        
        // Check if argument starts with "--"
        if (arg.substr(0, 2) == "--") {
            std::string key = arg.substr(2); // Remove the "--" prefix
            
            // Check if there's a following argument that doesn't start with "--"
            if (i + 1 < argc && argv[i + 1][0] != '-') {
                params[key] = argv[i + 1];
                i++; // Skip the next argument, as we've already processed it
            } else {
                // For flags without values, set to empty string
                params[key] = "";
            }
        }
    }
    
    return params;
}

class GurobiCallback : public GRBCallback {
private:
    std::string solution_pool_dir;
    std::ofstream& log;
    int& incumbent_count;
    std::chrono::time_point<std::chrono::high_resolution_clock> start_time;
    GRBModel* model_ptr;
    double best_obj; // Track best objective value
    bool is_minimization; // Direction of optimization
    
public:
    // Constructor
    GurobiCallback(std::string dir, std::ofstream& logStream, 
                  int& inc_count, 
                  std::chrono::time_point<std::chrono::high_resolution_clock> start,
                  GRBModel* model) 
    : solution_pool_dir(dir), log(logStream), incumbent_count(inc_count), 
      start_time(start), model_ptr(model) {
        // Determine optimization direction
        is_minimization = (model_ptr->get(GRB_IntAttr_ModelSense) == 1);
        // Initialize best objective based on optimization direction
        best_obj = is_minimization ? GRB_INFINITY : -GRB_INFINITY;
    }
    
protected:
    void callback() override {
        if (where == GRB_CB_MIPSOL) {
            // We have a new incumbent solution
            double obj = getDoubleInfo(GRB_CB_MIPSOL_OBJ);
            
            // Calculate elapsed time
            auto current_time = std::chrono::high_resolution_clock::now();
            auto elapsed = std::chrono::duration_cast<std::chrono::duration<double>>(current_time - start_time);
            double elapsed_seconds = elapsed.count();
            
            // Check if this solution is better than the best found so far
            bool is_better = false;
            if (is_minimization) {
                is_better = (obj < best_obj);
            } else {
                is_better = (obj > best_obj);
            }
            
            // Always log if solution is better, regardless of pool directory
            if (is_better) {
                best_obj = obj; // Update best objective
                // Simple log format: time: objective
                log << elapsed_seconds << ": " << obj << std::endl;
                
                // Print to console
                std::cout << "Improved incumbent found at " << elapsed_seconds 
                          << " seconds, objective: " << obj << std::endl;
            }
            
            // Only save to pool if a directory was provided
            if (!solution_pool_dir.empty()) {
                try {
                    // Write solution to a file in the solution pool directory
                    std::string solution_filename = generateSolutionFileName(
                        incumbent_count, obj, elapsed_seconds);
                    std::string full_path = solution_pool_dir + "/" + solution_filename;
                    
                    // Get the solution and write it to a file
                    std::ofstream sol_file(full_path);
                    if (sol_file.is_open()) {
                        // Get all variables
                        GRBVar* vars = model_ptr->getVars();
                        int numVars = model_ptr->get(GRB_IntAttr_NumVars);
                        
                        for (int j = 0; j < numVars; j++) {
                            GRBVar var = vars[j];
                            double value = getSolution(var);
                            // Only write non-zero binary variables
                            if (var.get(GRB_CharAttr_VType) == GRB_BINARY && fabs(value) > 0.5) {
                                sol_file << var.get(GRB_StringAttr_VarName) << " " << 1 << std::endl;
                            }
                        }
                        
                        delete[] vars;
                        sol_file.close();
                        std::cout << "Saved incumbent solution #" << incumbent_count + 1 
                            << " at " << elapsed_seconds << " seconds, objective: " << obj 
                            << " to: " << full_path << std::endl;
                    } else {
                        std::cerr << "Failed to create solution file: " << full_path << std::endl;
                    }
                    
                    incumbent_count++;
                } catch (GRBException& e) {
                    std::cerr << "Error saving incumbent solution: " << e.getMessage() << std::endl;
                }
            }
        }
    }
};

int main(int argc, char* argv[]) {
    try {
        // Parse command line arguments
        auto params = parseCommandLine(argc, argv);
        
        // Check for required parameters
        std::vector<std::string> requiredParams = {"lp_file", "output_file", "log_file"};
        bool missingParams = false;
        
        for (const auto& param : requiredParams) {
            if (params.find(param) == params.end()) {
                std::cerr << "Missing required parameter: --" << param << std::endl;
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
        
        // Get required parameter values
        std::string lp_file = params["lp_file"];
        std::string output_file = params["output_file"];
        std::string log_file = params["log_file"];
        
        // Optional parameters with defaults
        std::string mst_file = "";
        double time_limit = GRB_INFINITY;
        std::string solution_pool_dir = "";
        
        // Process optional parameters
        if (params.find("mst_file") != params.end()) {
            mst_file = params["mst_file"];
            // Verify the file exists
            if (!std::filesystem::exists(mst_file)) {
                std::cerr << "Warning: MST file does not exist: " << mst_file << std::endl;
                mst_file = ""; // Reset if it doesn't exist
            }
        }
        
        if (params.find("time_limit") != params.end()) {
            try {
                time_limit = std::stod(params["time_limit"]);
                std::cout << "Setting time limit to: " << time_limit << " seconds" << std::endl;
            } catch (const std::exception& e) {
                std::cerr << "Invalid time limit value: " << params["time_limit"] << std::endl;
                return 1;
            }
        }
        
        if (params.find("solution_pool_dir") != params.end()) {
            solution_pool_dir = params["solution_pool_dir"];
            // Create the directory if it doesn't exist
            if (!solution_pool_dir.empty()) {
                std::filesystem::create_directories(solution_pool_dir);
                std::cout << "Solution pool directory: " << solution_pool_dir << std::endl;
            }
        }
        
        // Open log file with simple format
        std::ofstream logStream(log_file);
        if (!logStream.is_open()) {
            std::cerr << "Failed to open log file: " << log_file << std::endl;
            return 1;
        }
        
        // Simple header for the log file - just time and objective values
        // logStream << "# Time(s): Objective" << std::endl;
        
        // Create Gurobi environment and model
        GRBEnv env = GRBEnv();
        GRBModel model = GRBModel(env, lp_file);
        
        // Set parameters
        if (time_limit != GRB_INFINITY) {
            model.set(GRB_DoubleParam_TimeLimit, time_limit);
        }
        
        // Make sure we capture all incumbent solutions
        model.set(GRB_IntParam_OutputFlag, 1);
        
        // Counter for incumbent solutions
        int incumbent_count = 0;
        auto start_time = std::chrono::high_resolution_clock::now();
        
        // Always set up callback for logging (even if no solution pool)
        GurobiCallback* callback = new GurobiCallback(solution_pool_dir, logStream, incumbent_count, start_time, &model);
        model.setCallback(callback);
        
        // Load initial solution from MST file if it exists
        if (!mst_file.empty()) {
            std::cout << "Loading initial solution from: " << mst_file << std::endl;
            model.read(mst_file);
        }
        
        // Optimize the model
        std::cout << "Starting optimization..." << std::endl;
        model.optimize();
        
        // Log runtime information
        auto end_time = std::chrono::high_resolution_clock::now();
        auto elapsed = std::chrono::duration_cast<std::chrono::duration<double>>(end_time - start_time);
        double total_runtime = elapsed.count();
        
        // Check optimization status
        int status = model.get(GRB_IntAttr_Status);
        if (status == GRB_OPTIMAL) {
            std::cout << "Optimal solution found!" << std::endl;
        } else if (status == GRB_TIME_LIMIT) {
            std::cout << "Time limit reached. Best solution found will be used." << std::endl;
        } else if (status == GRB_INFEASIBLE) {
            std::cout << "Model is infeasible." << std::endl;
        } else if (status == GRB_UNBOUNDED) {
            std::cout << "Model is unbounded." << std::endl;
        } else {
            std::cout << "Optimization ended with status: " << status << std::endl;
        }
        
        // Save the solution if a feasible solution was found
        if (model.get(GRB_IntAttr_SolCount) > 0) {
            double obj_val = model.get(GRB_DoubleAttr_ObjVal);
            std::cout << "Objective value: " << obj_val << std::endl;
            
            // Write solution to file
            model.write(output_file);
            std::cout << "Solution saved to: " << output_file << std::endl;
            
            // If we're using a solution pool directory, also save the final solution there
            if (!solution_pool_dir.empty()) {
                std::string final_solution_filename = generateSolutionFileName(
                    incumbent_count, obj_val, total_runtime);
                std::string full_path = solution_pool_dir + "/" + final_solution_filename;
                model.write(full_path);
                std::cout << "Final solution also saved to: " << full_path << std::endl;
            }
        } else {
            std::cout << "No solution found." << std::endl;
        }
        
        // Clean up callback if used
        if (callback != nullptr) {
            delete callback;
        }
        
        // Close log file
        logStream.close();
        
    } catch (GRBException e) {
        std::cerr << "Gurobi error code: " << e.getErrorCode() << std::endl;
        std::cerr << "Error message: " << e.getMessage() << std::endl;
        
        // Try to log the error
        std::ofstream errLog;
        try {
            if (argc > 1) {
                auto params = parseCommandLine(argc, argv);
                if (params.find("log_file") != params.end()) {
                    errLog.open(params["log_file"], std::ios_base::app);
                    if (errLog.is_open()) {
                        errLog << "Gurobi error code: " << e.getErrorCode() << std::endl;
                        errLog << "Error message: " << e.getMessage() << std::endl;
                        errLog.close();
                    }
                }
            }
        } catch (...) {
            // Ignore any errors during error logging
        }
        
        return 1;
    } catch (std::exception &e) {
        std::cerr << "Error: " << e.what() << std::endl;
        
        // Try to log the error
        std::ofstream errLog;
        try {
            if (argc > 1) {
                auto params = parseCommandLine(argc, argv);
                if (params.find("log_file") != params.end()) {
                    errLog.open(params["log_file"], std::ios_base::app);
                    if (errLog.is_open()) {
                        errLog << "Error: " << e.what() << std::endl;
                        errLog.close();
                    }
                }
            }
        } catch (...) {
            // Ignore any errors during error logging
        }
        
        return 1;
    }
    
    return 0;
}