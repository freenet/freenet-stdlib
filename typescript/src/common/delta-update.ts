// automatically generated by the FlatBuffers compiler, do not modify

/* eslint-disable @typescript-eslint/no-unused-vars, @typescript-eslint/no-explicit-any, @typescript-eslint/no-non-null-assertion */

import * as flatbuffers from 'flatbuffers';



export class DeltaUpdate implements flatbuffers.IUnpackableObject<DeltaUpdateT> {
  bb: flatbuffers.ByteBuffer|null = null;
  bb_pos = 0;
  __init(i:number, bb:flatbuffers.ByteBuffer):DeltaUpdate {
  this.bb_pos = i;
  this.bb = bb;
  return this;
}

static getRootAsDeltaUpdate(bb:flatbuffers.ByteBuffer, obj?:DeltaUpdate):DeltaUpdate {
  return (obj || new DeltaUpdate()).__init(bb.readInt32(bb.position()) + bb.position(), bb);
}

static getSizePrefixedRootAsDeltaUpdate(bb:flatbuffers.ByteBuffer, obj?:DeltaUpdate):DeltaUpdate {
  bb.setPosition(bb.position() + flatbuffers.SIZE_PREFIX_LENGTH);
  return (obj || new DeltaUpdate()).__init(bb.readInt32(bb.position()) + bb.position(), bb);
}

delta(index: number):number|null {
  const offset = this.bb!.__offset(this.bb_pos, 4);
  return offset ? this.bb!.readUint8(this.bb!.__vector(this.bb_pos + offset) + index) : 0;
}

deltaLength():number {
  const offset = this.bb!.__offset(this.bb_pos, 4);
  return offset ? this.bb!.__vector_len(this.bb_pos + offset) : 0;
}

deltaArray():Uint8Array|null {
  const offset = this.bb!.__offset(this.bb_pos, 4);
  return offset ? new Uint8Array(this.bb!.bytes().buffer, this.bb!.bytes().byteOffset + this.bb!.__vector(this.bb_pos + offset), this.bb!.__vector_len(this.bb_pos + offset)) : null;
}

static startDeltaUpdate(builder:flatbuffers.Builder) {
  builder.startObject(1);
}

static addDelta(builder:flatbuffers.Builder, deltaOffset:flatbuffers.Offset) {
  builder.addFieldOffset(0, deltaOffset, 0);
}

static createDeltaVector(builder:flatbuffers.Builder, data:number[]|Uint8Array):flatbuffers.Offset {
  builder.startVector(1, data.length, 1);
  for (let i = data.length - 1; i >= 0; i--) {
    builder.addInt8(data[i]!);
  }
  return builder.endVector();
}

static startDeltaVector(builder:flatbuffers.Builder, numElems:number) {
  builder.startVector(1, numElems, 1);
}

static endDeltaUpdate(builder:flatbuffers.Builder):flatbuffers.Offset {
  const offset = builder.endObject();
  builder.requiredField(offset, 4) // delta
  return offset;
}

static createDeltaUpdate(builder:flatbuffers.Builder, deltaOffset:flatbuffers.Offset):flatbuffers.Offset {
  DeltaUpdate.startDeltaUpdate(builder);
  DeltaUpdate.addDelta(builder, deltaOffset);
  return DeltaUpdate.endDeltaUpdate(builder);
}

unpack(): DeltaUpdateT {
  return new DeltaUpdateT(
    this.bb!.createScalarList<number>(this.delta.bind(this), this.deltaLength())
  );
}


unpackTo(_o: DeltaUpdateT): void {
  _o.delta = this.bb!.createScalarList<number>(this.delta.bind(this), this.deltaLength());
}
}

export class DeltaUpdateT implements flatbuffers.IGeneratedObject {
constructor(
  public delta: (number)[] = []
){}


pack(builder:flatbuffers.Builder): flatbuffers.Offset {
  const delta = DeltaUpdate.createDeltaVector(builder, this.delta);

  return DeltaUpdate.createDeltaUpdate(builder,
    delta
  );
}
}
