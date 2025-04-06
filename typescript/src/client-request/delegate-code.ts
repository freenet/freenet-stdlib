// automatically generated by the FlatBuffers compiler, do not modify

/* eslint-disable @typescript-eslint/no-unused-vars, @typescript-eslint/no-explicit-any, @typescript-eslint/no-non-null-assertion */

import * as flatbuffers from 'flatbuffers';



export class DelegateCode implements flatbuffers.IUnpackableObject<DelegateCodeT> {
  bb: flatbuffers.ByteBuffer|null = null;
  bb_pos = 0;
  __init(i:number, bb:flatbuffers.ByteBuffer):DelegateCode {
  this.bb_pos = i;
  this.bb = bb;
  return this;
}

static getRootAsDelegateCode(bb:flatbuffers.ByteBuffer, obj?:DelegateCode):DelegateCode {
  return (obj || new DelegateCode()).__init(bb.readInt32(bb.position()) + bb.position(), bb);
}

static getSizePrefixedRootAsDelegateCode(bb:flatbuffers.ByteBuffer, obj?:DelegateCode):DelegateCode {
  bb.setPosition(bb.position() + flatbuffers.SIZE_PREFIX_LENGTH);
  return (obj || new DelegateCode()).__init(bb.readInt32(bb.position()) + bb.position(), bb);
}

data(index: number):number|null {
  const offset = this.bb!.__offset(this.bb_pos, 4);
  return offset ? this.bb!.readUint8(this.bb!.__vector(this.bb_pos + offset) + index) : 0;
}

dataLength():number {
  const offset = this.bb!.__offset(this.bb_pos, 4);
  return offset ? this.bb!.__vector_len(this.bb_pos + offset) : 0;
}

dataArray():Uint8Array|null {
  const offset = this.bb!.__offset(this.bb_pos, 4);
  return offset ? new Uint8Array(this.bb!.bytes().buffer, this.bb!.bytes().byteOffset + this.bb!.__vector(this.bb_pos + offset), this.bb!.__vector_len(this.bb_pos + offset)) : null;
}

codeHash(index: number):number|null {
  const offset = this.bb!.__offset(this.bb_pos, 6);
  return offset ? this.bb!.readUint8(this.bb!.__vector(this.bb_pos + offset) + index) : 0;
}

codeHashLength():number {
  const offset = this.bb!.__offset(this.bb_pos, 6);
  return offset ? this.bb!.__vector_len(this.bb_pos + offset) : 0;
}

codeHashArray():Uint8Array|null {
  const offset = this.bb!.__offset(this.bb_pos, 6);
  return offset ? new Uint8Array(this.bb!.bytes().buffer, this.bb!.bytes().byteOffset + this.bb!.__vector(this.bb_pos + offset), this.bb!.__vector_len(this.bb_pos + offset)) : null;
}

static startDelegateCode(builder:flatbuffers.Builder) {
  builder.startObject(2);
}

static addData(builder:flatbuffers.Builder, dataOffset:flatbuffers.Offset) {
  builder.addFieldOffset(0, dataOffset, 0);
}

static createDataVector(builder:flatbuffers.Builder, data:number[]|Uint8Array):flatbuffers.Offset {
  builder.startVector(1, data.length, 1);
  for (let i = data.length - 1; i >= 0; i--) {
    builder.addInt8(data[i]!);
  }
  return builder.endVector();
}

static startDataVector(builder:flatbuffers.Builder, numElems:number) {
  builder.startVector(1, numElems, 1);
}

static addCodeHash(builder:flatbuffers.Builder, codeHashOffset:flatbuffers.Offset) {
  builder.addFieldOffset(1, codeHashOffset, 0);
}

static createCodeHashVector(builder:flatbuffers.Builder, data:number[]|Uint8Array):flatbuffers.Offset {
  builder.startVector(1, data.length, 1);
  for (let i = data.length - 1; i >= 0; i--) {
    builder.addInt8(data[i]!);
  }
  return builder.endVector();
}

static startCodeHashVector(builder:flatbuffers.Builder, numElems:number) {
  builder.startVector(1, numElems, 1);
}

static endDelegateCode(builder:flatbuffers.Builder):flatbuffers.Offset {
  const offset = builder.endObject();
  builder.requiredField(offset, 4) // data
  builder.requiredField(offset, 6) // code_hash
  return offset;
}

static createDelegateCode(builder:flatbuffers.Builder, dataOffset:flatbuffers.Offset, codeHashOffset:flatbuffers.Offset):flatbuffers.Offset {
  DelegateCode.startDelegateCode(builder);
  DelegateCode.addData(builder, dataOffset);
  DelegateCode.addCodeHash(builder, codeHashOffset);
  return DelegateCode.endDelegateCode(builder);
}

unpack(): DelegateCodeT {
  return new DelegateCodeT(
    this.bb!.createScalarList<number>(this.data.bind(this), this.dataLength()),
    this.bb!.createScalarList<number>(this.codeHash.bind(this), this.codeHashLength())
  );
}


unpackTo(_o: DelegateCodeT): void {
  _o.data = this.bb!.createScalarList<number>(this.data.bind(this), this.dataLength());
  _o.codeHash = this.bb!.createScalarList<number>(this.codeHash.bind(this), this.codeHashLength());
}
}

export class DelegateCodeT implements flatbuffers.IGeneratedObject {
constructor(
  public data: (number)[] = [],
  public codeHash: (number)[] = []
){}


pack(builder:flatbuffers.Builder): flatbuffers.Offset {
  const data = DelegateCode.createDataVector(builder, this.data);
  const codeHash = DelegateCode.createCodeHashVector(builder, this.codeHash);

  return DelegateCode.createDelegateCode(builder,
    data,
    codeHash
  );
}
}
