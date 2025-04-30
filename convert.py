from scipy.io import mmread, mmwrite
from scipy.sparse import csr_matrix, csc_matrix, save_npz
import anndata as ad
import os

dataset_num = 5
matrix_dir = f"test{dataset_num}"
matrix = mmread(f"{matrix_dir}/matrix.mtx")

os.makedirs(f"{matrix_dir}/other_formats", exist_ok=True)

csr = csr_matrix(matrix)
csc = csc_matrix(matrix)

save_npz(f"{matrix_dir}/other_formats/csr{dataset_num}.npz", csr)
save_npz(f"{matrix_dir}/other_formats/csc{dataset_num}.npz", csc)
# tested writing using mmwrite which just stores in mtx format text file anyway
# above format gets it smaller

matrix = ad.io.read_mtx(f"{matrix_dir}/matrix.mtx")

matrix.write_h5ad(f"{matrix_dir}/other_formats/matrix{dataset_num}.h5ad")
# matrix.write_h5ad(f"{matrix_dir}/other_formats/matrix{dataset_num}.h5ad.gz", compression='gzip')
# above does not compress as well as me just using 7zip to compress with gzip
matrix.write_loom(f"{matrix_dir}/other_formats/matrix{dataset_num}.loom")
