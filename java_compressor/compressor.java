package java_compressor;
import java.io.BufferedOutputStream;
import java.io.BufferedReader;
import java.io.DataOutputStream;
import java.io.FileOutputStream;
import java.io.FileReader;
import java.io.IOException;
import java.util.ArrayList;
import java.util.BitSet;

public class compressor {

    public static void main(String[] args) throws IOException {
        String line;
        String csvFile = "mex_matrix.csv";
        int dataRowCount = 0; 
        
        ArrayList<ArrayList<String>> allData = new ArrayList<>();
        int largestInt = 0;
        try (BufferedReader ba = new BufferedReader(new FileReader(csvFile))) {
            ba.readLine(); //gets rid of first row
            while ((line = ba.readLine()) != null) { // while there are more rows to read
                String[] values = line.split(",");
                dataRowCount++;
                for (int i = 3; i < values.length; i++){ //loop through each number in that row
                    if (Integer.valueOf(values[i]) > largestInt){
                        largestInt = Integer.valueOf(values[i]);
                    }
                }
                largestInt = Math.max(dataRowCount, Math.max(values.length, largestInt));
            }
        }
        ArrayList<String> currRowArrayList = new ArrayList<>(); //update this to be a bitset when capturing the numerical part of the matrix
        ArrayList<Integer> indexArrayList = new ArrayList<>();
        ArrayList<Integer> valuesArrayList = new ArrayList<>();
        int lenOfRow = 0;
        try (BufferedReader br = new BufferedReader(new FileReader(csvFile))) {
            String currRow = br.readLine();
            String[] values = currRow.split(",");
            for (String ele : values){ //add every element from row 1 into the current row's arraylist
                currRowArrayList.add(ele); 
            }
            allData.add(currRowArrayList); //add current row's arraylist to all of the data
            dataRowCount = 0;
            int numbersFound = 0;
            lenOfRow = values.length;
            while ((line = br.readLine()) != null) { // while there are more rows to read
                currRowArrayList = new ArrayList<>();
                dataRowCount++;
                values = line.split(",");
                currRowArrayList.add(values[0]); //feature type
                currRowArrayList.add(values[1]); //gene
                currRowArrayList.add(values[2]); //feature id
                for (int i = 3; i < values.length; i++){ //loop through each number in that row
                    if (!values[i].equals("0")){
                        indexArrayList.add(1);
                        valuesArrayList.add(Integer.parseInt(values[i]));
                    } else {
                        indexArrayList.add(0);
                    }
                    numbersFound++;
                }
                allData.add(currRowArrayList); //add current row's data to entire dataset arraylist
                System.out.println(); //print line for debugging
            }
        } catch (IOException e) {
            System.err.println("UH OH");
            e.printStackTrace();
        }

        //delta encode index arraylist
        int largestIndex = 0;
        int largestValue = 0;
        int currIndexValue = 0;
        int currValueValue = 0;
        ArrayList<Integer> deltaEncodedIndexArrayList = indexArrayList; 
        deltaEncodedIndexArrayList.add(indexArrayList.get(0));
        ArrayList<Integer> deltaEncodedValueArrayList = new ArrayList<>(); 
        deltaEncodedValueArrayList.add(valuesArrayList.get(0));
        // for (int i = 1; i < indexArrayList.size(); i++){
        //     currIndexValue = indexArrayList.get(i) - indexArrayList.get(i-1);
        //     deltaEncodedIndexArrayList.add(currIndexValue);
        //     // if (currIndexValue > largestIndex){
        //     //     largestIndex = currIndexValue;
        //     // }
        // }
        
        for (int i = 1; i < valuesArrayList.size(); i++) {
            currValueValue = valuesArrayList.get(i) - valuesArrayList.get(i-1);
            deltaEncodedValueArrayList.add(currValueValue);
            if (currValueValue > largestValue){
                largestValue = currValueValue;
            }
        }

        ////////////////////////////////////////////////output to .bin file//////////////////////////////////////////////////////////////
        DataOutputStream output = new DataOutputStream(new BufferedOutputStream(new FileOutputStream("java_compressor/our_compressed_matrix.bin")));
        System.out.println("Attempting to write matrix");
        output.writeInt(dataRowCount);
        output.writeInt(lenOfRow);
        output.writeInt(largestInt);
        output.writeInt(largestIndex);
        int rowCount = 0;
        for (ArrayList<String> ele : allData){
            if(rowCount == 0){
                output.writeChars(ele.toString());
                rowCount++;
                continue;
            }
            output.writeChars(ele.get(0));
            output.writeChars(ele.get(1));
            output.writeChars(ele.get(2));
            rowCount++;
        }
        
        
        
        int indexBitWidth = (int)(Math.floor(Math.log(1) / Math.log(2))) + 1;
        int valueBitWidth = (int)(Math.floor(Math.log(largestValue) / Math.log(2))) + 1;
        
        PackedIntVectorUsingBitSet indexPacked = new PackedIntVectorUsingBitSet(
            deltaEncodedIndexArrayList.size(), indexBitWidth
        );
        // for (int i = 0; i < deltaEncodedIndexArrayList.size(); i++) {
        //     indexPacked.set(i, deltaEncodedIndexArrayList.get(i));
        // }
        // output.write(indexPacked.getBitSet().toByteArray());
        
        PackedIntVectorUsingBitSet valuePacked = new PackedIntVectorUsingBitSet(
            deltaEncodedValueArrayList.size(), valueBitWidth
        );
        for (int i = 0; i < deltaEncodedValueArrayList.size(); i++) {
            valuePacked.set(i, deltaEncodedValueArrayList.get(i));
        }
        output.write(valuePacked.getBitSet().toByteArray());
        
        

        System.out.println("WRITTEN SUCCESSFULLY WOOHOO");
        output.close();
    }
}

