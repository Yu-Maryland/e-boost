// cuda_extractor.cu
#include <cuda_runtime.h>
#include <stdio.h>
#include <stdlib.h>
#include <assert.h>

// 这里用简单的常量表示“无穷大”cost
#define INFINITY_COST 1e9f

// 假设每个节点的 cost 是一个浮点数，
// 同时我们将原来的 HashMap cost_set 化简为：
//   每个节点仅保存一个最终的总 cost 和对应的 choice（这里用整型 id 表示）。
struct CostSet {
    float total;
    int   choice;
};

// 假设节点数据（扁平化后的 egraph 节点）
// 注意：这里仅给出一个简化示例，实际数据结构可能要包含更多信息
struct Node {
    int id;             // 节点 id
    int class_id;       // 节点所属的类 id
    float cost;         // 自身 cost
    int numChildren;    // 子节点数量
    int* children;      // 指向子节点 id 的数组（下标对应全局节点数组）
};

// 假设所有节点、costs 等数据已经保存在设备内存中
// 例如：d_nodes 数组、d_costs 数组、以及表示待处理节点索引的队列 d_pending_queue
// 以及每轮新待处理节点数量（用设备内存中的一个 int 保存）
  
// 一个示例 CUDA 内核，每个线程处理队列中的一个节点
__global__ void processNodesKernel(Node* d_nodes,
                                   int    numNodes,
                                   CostSet* d_costs,
                                   int* d_pending_queue, int pendingCount,
                                   int* d_new_queue, int* d_new_count)
{
    int idx = blockDim.x * blockIdx.x + threadIdx.x;
    if (idx >= pendingCount) return;
    
    // 取出待处理节点的下标
    int nodeIndex = d_pending_queue[idx];
    Node node = d_nodes[nodeIndex];
    
    // 判断所有子节点是否都已计算出 cost
    bool ready = true;
    for (int i = 0; i < node.numChildren; i++) {
        int childId = node.children[i];
        // 假设：如果子节点的 cost 为 INFINITY_COST，则表示还未更新完成
        if (d_costs[childId].total >= INFINITY_COST) {
            ready = false;
            break;
        }
    }
    if (!ready) return; // 还有子节点未处理，不更新该节点

    // 计算当前节点的新的 cost_set
    // 这里仅做个简单示例：假设新 cost = 自身 cost + 所有子 cost 之和
    float newCost = node.cost;
    for (int i = 0; i < node.numChildren; i++) {
        int childId = node.children[i];
        newCost += d_costs[childId].total;
    }
    
    // 取出原有 cost
    float prevCost = d_costs[nodeIndex].total;
    
    // 如果新 cost 更低，则更新该节点的 cost 并把“父节点”加入新的 pending queue
    if (newCost < prevCost) {
        d_costs[nodeIndex].total = newCost;
        d_costs[nodeIndex].choice = node.id; // 这里简单地将自身 id 作为 choice
        
        // TODO：如果需要更新父节点，则将父节点 id 写入新队列
        // 这里假设每个节点在内存中预先保存有指向父节点的数组（此处省略）
        // 为了示例，我们假设每个节点都有一个唯一的父节点（实际情况可能不止一个）
        // 举例：假设 node.parent 给出父节点的下标（如果有的话，不存在则为 -1）
        // 这里就不展开实现，实际情况中你需要预先构造好父节点列表。
        
        // 举例：如果有父节点，则原子写入到 d_new_queue 中：
        // int parentId = node.parent;
        // if (parentId >= 0) {
        //     int pos = atomicAdd(d_new_count, 1);
        //     d_new_queue[pos] = parentId;
        // }
    }
}

// 主机侧伪代码：调度内核，多轮迭代直到 pending queue 为空
int main() {
    // 1. 初始化、分配并拷贝数据到设备内存
    //    包括：d_nodes（节点数组）、d_costs（每个节点的 cost_set 数组）
    //         d_pending_queue（初始待处理节点下标队列）等。
    //
    // 此处省略具体数据加载的代码，假设你已经根据 egraph 结构完成了扁平化，
    // 并将所有节点信息存入设备内存数组 d_nodes，
    // 同时 d_costs 数组中初始值：叶节点 cost 为自身 cost，其余节点 cost 设置为 INFINITY_COST。
    
    // 2. 分配 pending 队列（两个队列用于交替调度）以及一个设备侧的 int 用于记录新队列长度
    int pendingQueueSize = /* 合适的大小 */;
    int *d_pending_queue, *d_new_queue, *d_new_count;
    cudaMalloc(&d_pending_queue, pendingQueueSize * sizeof(int));
    cudaMalloc(&d_new_queue, pendingQueueSize * sizeof(int));
    cudaMalloc(&d_new_count, sizeof(int));
    
    // 3. 将初始待处理节点 id 拷贝到 d_pending_queue，并设置 pendingCount
    int pendingCount = /* 初始队列节点数量 */;
    
    // 4. 循环调度内核，直到 pending 队列为空
    while (pendingCount > 0) {
        // 将 d_new_count 清零
        cudaMemset(d_new_count, 0, sizeof(int));
        
        int blockSize = 256;
        int gridSize = (pendingCount + blockSize - 1) / blockSize;
        processNodesKernel<<<gridSize, blockSize>>>(/* device pointers */ 
                                                    d_nodes, /* numNodes */  /*...*/, d_costs,
                                                    d_pending_queue, pendingCount,
                                                    d_new_queue, d_new_count);
        cudaDeviceSynchronize();
        
        // 将 d_new_count 拷贝到 host 以获得新队列大小
        int newCount = 0;
        cudaMemcpy(&newCount, d_new_count, sizeof(int), cudaMemcpyDeviceToHost);
        
        // 交换 d_pending_queue 和 d_new_queue 指针，准备下一轮迭代
        int* tmp = d_pending_queue;
        d_pending_queue = d_new_queue;
        d_new_queue = tmp;
        pendingCount = newCount;
    }
    
    // 5. 处理完毕后，将最终的 cost_set 结果从 d_costs 拷贝到主机进行后续处理
    // 6. 清理设备内存
    cudaFree(d_nodes);
    cudaFree(d_costs);
    cudaFree(d_pending_queue);
    cudaFree(d_new_queue);
    cudaFree(d_new_count);
    
    return 0;
}
