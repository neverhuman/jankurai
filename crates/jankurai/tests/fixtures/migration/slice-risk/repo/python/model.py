import multiprocessing
import numpy as np
import torch


def load_model(path: str):
    checkpoint = torch.load(path)
    np.random.seed(0)
    pool = multiprocessing.Pool()
    return checkpoint, pool
