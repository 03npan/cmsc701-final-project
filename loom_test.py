import h5py

# verify loom files are correctly formatted

dataset_num = 3
matrix_dir = f"test{dataset_num}"
with h5py.File(f"{matrix_dir}/other_versions/matrix{dataset_num}.loom", 'r') as file:
    # Check the structure of the file
    print("File structure:")
    def print_name(name):
        print(name)
    
    # Print all groups and datasets inside the file
    file.visit(print_name)