// automatically generated by the FlatBuffers compiler, do not modify

/* eslint-disable @typescript-eslint/no-unused-vars, @typescript-eslint/no-explicit-any, @typescript-eslint/no-non-null-assertion */

import * as flatbuffers from 'flatbuffers';

import { ContractInstanceId, ContractInstanceIdT } from '../common/contract-instance-id.js';


export class RelatedDeltaUpdate implements flatbuffers.IUnpackableObject<RelatedDeltaUpdateT> {
  bb: flatbuffers.ByteBuffer|null = null;
  bb_pos = 0;
  __init(i:number, bb:flatbuffers.ByteBuffer):RelatedDeltaUpdate {
  this.bb_pos = i;
  this.bb = bb;
  return this;
}

static getRootAsRelatedDeltaUpdate(bb:flatbuffers.ByteBuffer, obj?:RelatedDeltaUpdate):RelatedDeltaUpdate {
  return (obj || new RelatedDeltaUpdate()).__init(bb.readInt32(bb.position()) + bb.position(), bb);
}

static getSizePrefixedRootAsRelatedDeltaUpdate(bb:flatbuffers.ByteBuffer, obj?:RelatedDeltaUpdate):RelatedDeltaUpdate {
  bb.setPosition(bb.position() + flatbuffers.SIZE_PREFIX_LENGTH);
  return (obj || new RelatedDeltaUpdate()).__init(bb.readInt32(bb.position()) + bb.position(), bb);
}

relatedTo(obj?:ContractInstanceId):ContractInstanceId|null {
  const offset = this.bb!.__offset(this.bb_pos, 4);
  return offset ? (obj || new ContractInstanceId()).__init(this.bb!.__indirect(this.bb_pos + offset), this.bb!) : null;
}

delta(index: number):number|null {
  const offset = this.bb!.__offset(this.bb_pos, 6);
  return offset ? this.bb!.readUint8(this.bb!.__vector(this.bb_pos + offset) + index) : 0;
}

deltaLength():number {
  const offset = this.bb!.__offset(this.bb_pos, 6);
  return offset ? this.bb!.__vector_len(this.bb_pos + offset) : 0;
}

deltaArray():Uint8Array|null {
  const offset = this.bb!.__offset(this.bb_pos, 6);
  return offset ? new Uint8Array(this.bb!.bytes().buffer, this.bb!.bytes().byteOffset + this.bb!.__vector(this.bb_pos + offset), this.bb!.__vector_len(this.bb_pos + offset)) : null;
}

static startRelatedDeltaUpdate(builder:flatbuffers.Builder) {
  builder.startObject(2);
}

static addRelatedTo(builder:flatbuffers.Builder, relatedToOffset:flatbuffers.Offset) {
  builder.addFieldOffset(0, relatedToOffset, 0);
}

static addDelta(builder:flatbuffers.Builder, deltaOffset:flatbuffers.Offset) {
  builder.addFieldOffset(1, deltaOffset, 0);
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

static endRelatedDeltaUpdate(builder:flatbuffers.Builder):flatbuffers.Offset {
  const offset = builder.endObject();
  builder.requiredField(offset, 4) // related_to
  builder.requiredField(offset, 6) // delta
  return offset;
}

static createRelatedDeltaUpdate(builder:flatbuffers.Builder, relatedToOffset:flatbuffers.Offset, deltaOffset:flatbuffers.Offset):flatbuffers.Offset {
  RelatedDeltaUpdate.startRelatedDeltaUpdate(builder);
  RelatedDeltaUpdate.addRelatedTo(builder, relatedToOffset);
  RelatedDeltaUpdate.addDelta(builder, deltaOffset);
  return RelatedDeltaUpdate.endRelatedDeltaUpdate(builder);
}

unpack(): RelatedDeltaUpdateT {
  return new RelatedDeltaUpdateT(
    (this.relatedTo() !== null ? this.relatedTo()!.unpack() : null),
    this.bb!.createScalarList<number>(this.delta.bind(this), this.deltaLength())
  );
}


unpackTo(_o: RelatedDeltaUpdateT): void {
  _o.relatedTo = (this.relatedTo() !== null ? this.relatedTo()!.unpack() : null);
  _o.delta = this.bb!.createScalarList<number>(this.delta.bind(this), this.deltaLength());
}
}

export class RelatedDeltaUpdateT implements flatbuffers.IGeneratedObject {
constructor(
  public relatedTo: ContractInstanceIdT|null = null,
  public delta: (number)[] = []
){}


pack(builder:flatbuffers.Builder): flatbuffers.Offset {
  const relatedTo = (this.relatedTo !== null ? this.relatedTo!.pack(builder) : 0);
  const delta = RelatedDeltaUpdate.createDeltaVector(builder, this.delta);

  return RelatedDeltaUpdate.createRelatedDeltaUpdate(builder,
    relatedTo,
    delta
  );
}
}
