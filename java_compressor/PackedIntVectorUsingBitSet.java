package java_compressor;
import java.util.BitSet;
public class PackedIntVectorUsingBitSet {
    private BitSet packedBitSet;
    private int bitsPerElement;
    private int packedBitSetSize;

    public PackedIntVectorUsingBitSet(int inputSize, int inputBitsPerElement){
        this.bitsPerElement = inputBitsPerElement;
        this.packedBitSetSize = inputSize;
        this.packedBitSet = new BitSet(packedBitSetSize * bitsPerElement);
    }

    public void set(int indexToSet, int value) {
        int startingBit = indexToSet * bitsPerElement;
        for (int i = 0; i < bitsPerElement; i++) {
            if (((value >> i) & 1) == 1){
                packedBitSet.set(startingBit + i);
            }
        }
    }

    public int get(int indexToSet){
        int startingBit = indexToSet * bitsPerElement;
        int value = 0;
        for (int i = 0; i < bitsPerElement; i++) {
            if(packedBitSet.get(startingBit + i)){
                value |= (1 << i);
            }
        }
        return value;
    }

    public BitSet getBitSet() {
        return packedBitSet;
    }

    public void addBits(BitSet valueOf) {
        for (int i = 0; i < valueOf.length(); i++){
            if(valueOf.get(i)){
                this.packedBitSet.set(i);
            }
        }
    }
}

