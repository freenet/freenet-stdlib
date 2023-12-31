// automatically generated by the FlatBuffers compiler, do not modify

/* eslint-disable @typescript-eslint/no-unused-vars, @typescript-eslint/no-explicit-any, @typescript-eslint/no-non-null-assertion */

import * as flatbuffers from 'flatbuffers';



export class GenerateRandData implements flatbuffers.IUnpackableObject<GenerateRandDataT> {
  bb: flatbuffers.ByteBuffer|null = null;
  bb_pos = 0;
  __init(i:number, bb:flatbuffers.ByteBuffer):GenerateRandData {
  this.bb_pos = i;
  this.bb = bb;
  return this;
}

static getRootAsGenerateRandData(bb:flatbuffers.ByteBuffer, obj?:GenerateRandData):GenerateRandData {
  return (obj || new GenerateRandData()).__init(bb.readInt32(bb.position()) + bb.position(), bb);
}

static getSizePrefixedRootAsGenerateRandData(bb:flatbuffers.ByteBuffer, obj?:GenerateRandData):GenerateRandData {
  bb.setPosition(bb.position() + flatbuffers.SIZE_PREFIX_LENGTH);
  return (obj || new GenerateRandData()).__init(bb.readInt32(bb.position()) + bb.position(), bb);
}

data():number {
  const offset = this.bb!.__offset(this.bb_pos, 4);
  return offset ? this.bb!.readInt32(this.bb_pos + offset) : 0;
}

static startGenerateRandData(builder:flatbuffers.Builder) {
  builder.startObject(1);
}

static addData(builder:flatbuffers.Builder, data:number) {
  builder.addFieldInt32(0, data, 0);
}

static endGenerateRandData(builder:flatbuffers.Builder):flatbuffers.Offset {
  const offset = builder.endObject();
  return offset;
}

static createGenerateRandData(builder:flatbuffers.Builder, data:number):flatbuffers.Offset {
  GenerateRandData.startGenerateRandData(builder);
  GenerateRandData.addData(builder, data);
  return GenerateRandData.endGenerateRandData(builder);
}

unpack(): GenerateRandDataT {
  return new GenerateRandDataT(
    this.data()
  );
}


unpackTo(_o: GenerateRandDataT): void {
  _o.data = this.data();
}
}

export class GenerateRandDataT implements flatbuffers.IGeneratedObject {
constructor(
  public data: number = 0
){}


pack(builder:flatbuffers.Builder): flatbuffers.Offset {
  return GenerateRandData.createGenerateRandData(builder,
    this.data
  );
}
}
