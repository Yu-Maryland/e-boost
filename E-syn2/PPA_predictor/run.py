import json
import torch
import dgl
from sklearn.preprocessing import OneHotEncoder

# Assuming the feature builder and dataset classes are the same as in `parse_json`
from parse_json import EGraphDataset, feature_embedding_builder

# Function to load models - replace with the actual model loading if different
def load_regression_model(model_path):
    # Load regression model - replace with the actual code
    model = torch.load(model_path)
    return model

def load_gnn_model(model_path):
    # Load the GNN model - replace with the actual code
    model = torch.load(model_path)
    model.eval()  # Set the model to evaluation mode
    return model

# Function to encode and predict with the regression model
def predict_regression(graph, model):
    # Extract features and encode the graph for the regression model
    # Replace with actual feature extraction and graph encoding for your regression model
    features = ...  # Extracted features
    prediction = model.predict(features)
    return prediction

# Function to encode and predict with the GNN model
def predict_gnn(graph_dataset, model):
    # Graph is already encoded in graph_dataset, just make predictions
    for g in graph_dataset.graphs:
        prediction = model(g, g.ndata['op'])
        # Perform any necessary post-processing on prediction if required
    return prediction

if __name__ == "__main__":
    # Load the trained models
    regression_model = load_regression_model('path_to_regression_model.pt')
    gnn_model = load_gnn_model('path_to_gnn_model.pt')

    # Path to the new JSON file containing the graph for inference
    json_file_path = 'path_to_new_json_file.json'

    # Load and preprocess the graph data
    graph_dataset = EGraphDataset(
        file_path=json_file_path,
        additional_feature_builder=feature_embedding_builder
    )

    # Make predictions using the regression model
    # Note: The graph object should be processed to match the input expected by the regression model
    reg_prediction = predict_regression(graph_dataset.graphs[0], regression_model)
    print("Regression Model Prediction:", reg_prediction)

    # Make predictions using the GNN model
    gnn_prediction = predict_gnn(graph_dataset, gnn_model)
    print("GNN Model Prediction:", gnn_prediction)