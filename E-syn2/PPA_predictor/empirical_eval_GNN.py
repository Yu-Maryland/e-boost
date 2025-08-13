import torch
import torch.nn as nn
import torch.nn.functional as F
import dgl
import dgl.function as fn
from dgl.dataloading import GraphDataLoader
from sklearn.model_selection import train_test_split
from models4GNN import GCNModel, GATModel, GraphSAGEModel, APPNPModel, GINModel
import optuna

# Load the dataset
dataset = dgl.load_graphs('./graph_dataset.bin')[0]

# Split the dataset into training and testing sets
train_graphs, test_graphs = train_test_split(dataset, test_size=0.2, random_state=42)

# Create dataloaders
train_dataloader = GraphDataLoader(train_graphs, batch_size=32, shuffle=True)
test_dataloader = GraphDataLoader(test_graphs, batch_size=32)

# Define the objective function for Optuna
def objective(trial, model_name):
    # Define hyperparameters to tune for each model
    if model_name == 'GCN':
        hid_feats = trial.suggest_int('hid_feats', 8, 64)
        model = GCNModel(train_graphs[0].ndata['op'].shape[1], hid_feats, 1)
    elif model_name == 'GAT':
        hid_feats = trial.suggest_int('hid_feats', 4, 32)
        num_heads = trial.suggest_int('num_heads', 1, 8)
        model = GATModel(train_graphs[0].ndata['op'].shape[1], hid_feats, 1, num_heads)
    elif model_name == 'GraphSAGE':
        hid_feats = trial.suggest_int('hid_feats', 8, 64)
        model = GraphSAGEModel(train_graphs[0].ndata['op'].shape[1], hid_feats, 1)
    elif model_name == 'APPNP':
        hid_feats = trial.suggest_int('hid_feats', 8, 64)
        k = trial.suggest_int('k', 1, 10)
        alpha = trial.suggest_float('alpha', 0.1, 0.9)
        model = APPNPModel(train_graphs[0].ndata['op'].shape[1], hid_feats, 1, k, alpha)
    elif model_name == 'GIN':
        hid_feats = trial.suggest_int('hid_feats', 8, 64)
        num_layers = trial.suggest_int('num_layers', 1, 5)
        model = GINModel(train_graphs[0].ndata['op'].shape[1], hid_feats, 1, num_layers)

    loss_func = nn.MSELoss()
    optimizer = torch.optim.Adam(model.parameters(), lr=0.01)

    # Training loop
    for epoch in range(100):
        model.train()
        for batched_graph in train_dataloader:
            pred = model(batched_graph, batched_graph.ndata['op'].float())
            loss = loss_func(pred, batched_graph.ndata['labels'].float())
            optimizer.zero_grad()
            loss.backward()
            optimizer.step()

    # Evaluation
    model.eval()
    total_error = 0
    for batched_graph in test_dataloader:
        pred = model(batched_graph, batched_graph.ndata['op'].float())
        total_error += loss_func(pred, batched_graph.ndata['labels'].float()).item()

    mean_error = total_error / len(test_dataloader)
    return mean_error

# Define the loss function
loss_func = nn.MSELoss()

# Models and hyperparameter tuning
models = ['GCN', 'GAT', 'GraphSAGE', 'APPNP', 'GIN']

best_models = {}
for model_name in models:
    study = optuna.create_study(direction='minimize')
    study.optimize(lambda trial: objective(trial, model_name), n_trials=100)
    best_params = study.best_params
    best_model = None

    if model_name == 'GCN':
        best_model = GCNModel(train_graphs[0].ndata['op'].shape[1], best_params['hid_feats'], 1)
    elif model_name == 'GAT':
        best_model = GATModel(train_graphs[0].ndata['op'].shape[1], best_params['hid_feats'], 1, best_params['num_heads'])
    elif model_name == 'GraphSAGE':
        best_model = GraphSAGEModel(train_graphs[0].ndata['op'].shape[1], best_params['hid_feats'], 1)
    elif model_name == 'APPNP':
        best_model = APPNPModel(train_graphs[0].ndata['op'].shape[1], best_params['hid_feats'], 1, best_params['k'], best_params['alpha'])
    elif model_name == 'GIN':
        best_model = GINModel(train_graphs[0].ndata['op'].shape[1], best_params['hid_feats'], 1, best_params['num_layers'])

    best_models[model_name] = best_model

# Compare the best models
for model_name, model in best_models.items():
    total_error = 0
    for batched_graph in test_dataloader:
        pred = model(batched_graph, batched_graph.ndata['op'].float())
        total_error += loss_func(pred, batched_graph.ndata['labels'].float()).item()
    
    mean_error = total_error / len(test_dataloader)
    print(f'{model_name} - Mean Squared Error on test set: {mean_error:.4f}')