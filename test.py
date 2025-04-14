import scipy.io
import csv

matrix_dir = "test"
mat = scipy.io.mmread(f"{matrix_dir}/matrix.mtx")
with open(f"{matrix_dir}/features.tsv") as f:
    feature_ids = [row[0] for row in csv.reader(f, delimiter="\t")]
with open(f"{matrix_dir}/features.tsv") as f:
    gene_names = [row[1] for row in csv.reader(f, delimiter="\t")]
with open(f"{matrix_dir}/features.tsv") as f:
    feature_types = [row[2] for row in csv.reader(f, delimiter="\t")]
with open(f"{matrix_dir}/barcodes.tsv") as f:
    barcodes = [row[0] for row in csv.reader(f, delimiter="\t")]

import pandas as pd
matrix = pd.DataFrame.sparse.from_spmatrix(mat)
matrix.to_csv("mex_matrix_no_headers.csv", index=False)
matrix.columns = barcodes
matrix.insert(loc=0, column="feature_id", value=feature_ids)
matrix.insert(loc=0, column="gene", value=gene_names)
matrix.insert(loc=0, column="feature_type", value=feature_types)
 
# display matrix
# print(matrix)
# save the table as a CSV (note the CSV will be a very large file)
matrix.to_csv("mex_matrix.csv", index=False)
