import dgl

def load_dataset(file_path):
    graphs, label_dict = dgl.data.utils.load_graphs(file_path)
    labels = label_dict['labels']
    return graphs, labels

# Assuming the dataset file is 'graph_dataset.bin'
graphs, labels = load_dataset('graph_dataset.bin')

