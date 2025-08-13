import dgl
import torch
import torch.nn as nn
import torch.nn.functional as F
from dgl.nn import GINConv

class GCNModel(nn.Module):
    def __init__(self, in_feats, hid_feats, out_feats):
        super(GCNModel, self).__init__()
        self.conv1 = dgl.nn.GraphConv(in_feats, hid_feats)
        self.conv2 = dgl.nn.GraphConv(hid_feats, out_feats)

    def forward(self, g, in_feat):
        h = self.conv1(g, in_feat)
        h = F.relu(h)
        h = self.conv2(g, h)
        g.ndata['h'] = h
        return dgl.mean_nodes(g, 'h')

class GATModel(nn.Module):
    def __init__(self, in_feats, hid_feats, out_feats, num_heads):
        super(GATModel, self).__init__()
        self.conv1 = dgl.nn.GATConv(in_feats, hid_feats, num_heads)
        self.conv2 = dgl.nn.GATConv(hid_feats * num_heads, out_feats, 1)

    def forward(self, g, in_feat):
        h = self.conv1(g, in_feat).flatten(1)
        h = F.relu(h)
        h = self.conv2(g, h).mean(1)
        g.ndata['h'] = h
        return dgl.mean_nodes(g, 'h')

class GraphSAGEModel(nn.Module):
    def __init__(self, in_feats, hid_feats, out_feats):
        super(GraphSAGEModel, self).__init__()
        self.conv1 = dgl.nn.SAGEConv(in_feats, hid_feats, 'mean')
        self.conv2 = dgl.nn.SAGEConv(hid_feats, out_feats, 'mean')

    def forward(self, g, in_feat):
        h = self.conv1(g, in_feat)
        h = F.relu(h)
        h = self.conv2(g, h)
        g.ndata['h'] = h
        return dgl.mean_nodes(g, 'h')

class APPNPModel(nn.Module):
    def __init__(self, in_feats, hid_feats, out_feats, k, alpha):
        super(APPNPModel, self).__init__()
        self.lin1 = nn.Linear(in_feats, hid_feats)
        self.lin2 = nn.Linear(hid_feats, out_feats)
        self.prop1 = dgl.nn.APPNPConv(k, alpha)

    def forward(self, g, in_feat):
        h = self.lin1(in_feat)
        h = F.relu(h)
        h = self.lin2(h)
        h = self.prop1(g, h)
        return h

class GINModel(nn.Module):
    def __init__(self, input_dim, hidden_dim, output_dim, num_layers):
        super(GINModel, self).__init__()
        self.layers = nn.ModuleList()
        self.batch_norms = nn.ModuleList()

        for i in range(num_layers):
            if i == 0:
                mlp = nn.Sequential(
                    nn.Linear(input_dim, hidden_dim),
                    nn.ReLU(),
                    nn.Linear(hidden_dim, hidden_dim)
                )
            else:
                mlp = nn.Sequential(
                    nn.Linear(hidden_dim, hidden_dim),
                    nn.ReLU(),
                    nn.Linear(hidden_dim, hidden_dim)
                )
            self.layers.append(GINConv(mlp, 'mean'))
            self.batch_norms.append(nn.BatchNorm1d(hidden_dim))

        self.linear_pred = nn.Linear(hidden_dim, output_dim)

    def forward(self, g, features):
        h = features
        for layer, batch_norm in zip(self.layers, self.batch_norms):
            h = layer(g, h)
            h = batch_norm(h)
            h = F.relu(h)
        g.ndata['h'] = h
        hg = dgl.mean_nodes(g, 'h')
        return self.linear_pred(hg)

def save_model(model, filename):
    torch.save(model.state_dict(), filename)

def load_model(model, filename):
    model.load_state_dict(torch.load(filename))
    model.eval()
    return model