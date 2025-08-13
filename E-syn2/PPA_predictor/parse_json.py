import json
import networkx as nx
import sys

import dgl
import json
import torch
import os
from sklearn.preprocessing import OneHotEncoder

# Function to load a JSON file to a networkx graph - For Regression use
def load_json_to_networkx(file_path):
    # Read the JSON file
    with open(file_path, 'r') as f:
        data = json.load(f)
    
    # Create a directed graph
    G = nx.DiGraph()
    
    # Add nodes with attributes to the graph
    for node_id, node_data in data['nodes'].items():
        G.add_node(node_id, **node_data)
    
    # Add edges based on children
    for node_id, node_data in data['nodes'].items():
        for child_id in node_data['children']:
            G.add_edge(node_id, child_id)
    
    # Optionally: return the root eclasses as well
    root_eclasses = data.get('root_eclasses', [])
    
    return G, root_eclasses

# Function to load a JSON file to a DGL graph - For Graph Learning based PPA prediction
class EGraphDataset:
    def __init__(self, file_path, additional_feature_builder=None):
        self.file_path = file_path
        self.graphs = []
        self.labels = []
        self.additional_feature_builder = additional_feature_builder
        self.load_data()

    def load_data(self):
        with open(self.file_path, 'r') as f:
            data = json.load(f)
        
        # Extract node features and labels
        nodes_data = data['nodes']
        node_ids = list(nodes_data.keys())
        node_ops = [nodes_data[n_id]['op'] for n_id in node_ids]
        
        # One-hot encode the 'op' feature
        one_hot_encoder = OneHotEncoder(sparse=False)
        ops_encoded = one_hot_encoder.fit_transform([[op] for op in node_ops])
        ops_encoded = torch.tensor(ops_encoded).float()
        
        # Create an empty DGL graph
        g = dgl.DGLGraph()
        g.add_nodes(len(node_ids))
        
        # Add edges
        for node_id, node_data in nodes_data.items():
            for child_id in node_data['children']:
                # DGL graphs are directional by default
                g.add_edges(node_ids.index(node_id), node_ids.index(child_id))
        
        # Add the one-hot encoded 'op' features to the graph
        g.ndata['op'] = ops_encoded

        # If additional features are provided by the user via the feature builder
        if self.additional_feature_builder:
            additional_features = self.additional_feature_builder(node_ids, nodes_data)
            for key, values in additional_features.items():
                g.ndata[key] = torch.tensor(values).float()
        
        # set labels as PPA - currently set to 100
        self.labels = 100
        
        self.graphs.append(g)

    def save(self, save_path):
        # Make sure the save directory exists
        os.makedirs(os.path.dirname(save_path), exist_ok=True)
        
        # Ensure labels are a tensor
        labels_tensor = torch.tensor(self.labels) if self.labels else torch.tensor([])

        # Serialize the graphs and labels to the specified file
        with open(save_path, 'wb') as f:
            dgl.save_graphs(save_path, self.graphs, {'labels': labels_tensor})

# Additional feature builder function
def feature_embedding_builder(node_ids, nodes_data):
    return {
        'cost': [nodes_data[n_id]['cost'] for n_id in node_ids],
        # Add more features here
    }

if __name__ == "__main__":
    # Check if a filepath was provided as a command line argument
    if len(sys.argv) != 2:
        print("Usage: python script.py <path_to_json_file>")
        sys.exit(1)

    file_path = sys.argv[1]

    # Use the function to create the networkx graph
    graph, root_eclasses = load_json_to_networkx(file_path)

    # Now you can work with the 'graph' object using networkx functions
    #print("Nodes in the graph:", graph.nodes(data=True))
    #print("Edges in the graph:", graph.edges())
    #print("Root eclasses:", root_eclasses)
    
    # print the infomations for edge count, node count, and degree distribution
    print("Edge count:", graph.number_of_edges())
    print("Node count:", graph.number_of_nodes())
    print("Degree distribution:", nx.degree_histogram(graph))
    
    # Assuming your JSON file is called 'graph_data.json' and the output file is 'graph_dataset.bin'
    dataset = EGraphDataset(file_path=file_path, additional_feature_builder=feature_embedding_builder)
    dataset.save('./graph_dataset.bin')