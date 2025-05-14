import h5py
from scipy.io import mmread
from scipy.sparse import csc_matrix
import os

dataset_num = 3
matrix_dir = f"atlas{dataset_num}"
matrix = mmread(f"{matrix_dir}/matrix.mtx")
sparse_matrix = csc_matrix(matrix)

os.makedirs(f"{matrix_dir}/other_formats", exist_ok=True)

file_path = f'{matrix_dir}/other_formats/atlas{dataset_num}.h5'

with h5py.File(file_path, 'w') as hf:
    group = hf.create_group('matrix')
    group.create_dataset('data', data=sparse_matrix.data)
    group.create_dataset('indices', data=sparse_matrix.indices)
    group.create_dataset('indptr', data=sparse_matrix.indptr)
    group.attrs['shape'] = sparse_matrix.shape
    group.attrs['format'] = 'csr'

# don't have barcodes and features info to add but those data
# are practically negligible in terms of size

"""
File structure:
matrix
matrix/barcodes
matrix/data
matrix/features
matrix/features/_all_tag_keys
matrix/features/feature_type
matrix/features/genome
matrix/features/id
matrix/features/name
matrix/indices
matrix/indptr
matrix/shape
"""