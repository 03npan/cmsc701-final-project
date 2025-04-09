import java.io.BufferedOutputStream;
import java.io.BufferedReader;
import java.io.DataOutputStream;
import java.io.FileOutputStream;
import java.io.FileReader;
import java.io.IOException;
import java.util.ArrayList;

public class compressor {
    public static void main(String[] args) throws IOException {
        String line;
        String csvFile = "mex_matrix.csv";
        int dataRowCount = 0; 
        ArrayList<ArrayList<String>> allData = new ArrayList<>();

        try (BufferedReader br = new BufferedReader(new FileReader(csvFile))) {
            String currRow = br.readLine();
            ArrayList<String> currRowArrayList = new ArrayList<>(); //update this to be a bitset when capturing the numerical part of the matrix
            String[] values = currRow.split(",");
            for (String ele : values){ //add every element from row 1 into the current row's arraylist
                currRowArrayList.add(ele); 
            }
            allData.add(currRowArrayList); //add current row's arraylist to all of the data
            while ((line = br.readLine()) != null) { // while there are more rows to read
                currRowArrayList = new ArrayList<>();
                dataRowCount++;
                values = line.split(",");
                currRowArrayList.add(values[0]); //feature type
                currRowArrayList.add(values[1]); //gene
                currRowArrayList.add(values[2]); //feature id
                for (int i = 3; i < values.length; i++){ //loop through each number in that row
                    if (!values[i].equals("0")){
                        // currRowArrayList.add(String.valueOf(dataRowCount)); //row - WE DO NOT NEED TO WRITE ROW NUMBER BECAUSE WE ARE WRITING EVERY ROW SO IT IS EASY TO LATER DETERMINE WHICH ROW NUMBER WE ARE IN
                        currRowArrayList.add(String.valueOf(i)); //column
                        currRowArrayList.add(values[i]); //value
                    }
                }
                allData.add(currRowArrayList); //add current row's data to entire dataset arraylist
                System.out.println(); //print line for debugging
            }
        } catch (IOException e) {
            System.err.println("UH OH");
            e.printStackTrace();
        }


        ////////////////////////////////////////////////output to .bin file//////////////////////////////////////////////////////////////
        DataOutputStream output = new DataOutputStream(new BufferedOutputStream(new FileOutputStream("our_compressed_matrix.bin")));
        System.out.println("Attempting to write matrix");

        output.writeChars(allData.toString()); 

        System.out.println("WRITTEN SUCCESSFULLY WOOHOO");
        output.close();
    }
}

