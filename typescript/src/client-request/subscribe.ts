// automatically generated by the FlatBuffers compiler, do not modify

/* eslint-disable @typescript-eslint/no-unused-vars, @typescript-eslint/no-explicit-any, @typescript-eslint/no-non-null-assertion */

import * as flatbuffers from 'flatbuffers';

import { ContractKey, ContractKeyT } from '../common/contract-key.js';


export class Subscribe implements flatbuffers.IUnpackableObject<SubscribeT> {
  bb: flatbuffers.ByteBuffer|null = null;
  bb_pos = 0;
  __init(i:number, bb:flatbuffers.ByteBuffer):Subscribe {
  this.bb_pos = i;
  this.bb = bb;
  return this;
}

static getRootAsSubscribe(bb:flatbuffers.ByteBuffer, obj?:Subscribe):Subscribe {
  return (obj || new Subscribe()).__init(bb.readInt32(bb.position()) + bb.position(), bb);
}

static getSizePrefixedRootAsSubscribe(bb:flatbuffers.ByteBuffer, obj?:Subscribe):Subscribe {
  bb.setPosition(bb.position() + flatbuffers.SIZE_PREFIX_LENGTH);
  return (obj || new Subscribe()).__init(bb.readInt32(bb.position()) + bb.position(), bb);
}

key(obj?:ContractKey):ContractKey|null {
  const offset = this.bb!.__offset(this.bb_pos, 4);
  return offset ? (obj || new ContractKey()).__init(this.bb!.__indirect(this.bb_pos + offset), this.bb!) : null;
}

summary(index: number):number|null {
  const offset = this.bb!.__offset(this.bb_pos, 6);
  return offset ? this.bb!.readUint8(this.bb!.__vector(this.bb_pos + offset) + index) : 0;
}

summaryLength():number {
  const offset = this.bb!.__offset(this.bb_pos, 6);
  return offset ? this.bb!.__vector_len(this.bb_pos + offset) : 0;
}

summaryArray():Uint8Array|null {
  const offset = this.bb!.__offset(this.bb_pos, 6);
  return offset ? new Uint8Array(this.bb!.bytes().buffer, this.bb!.bytes().byteOffset + this.bb!.__vector(this.bb_pos + offset), this.bb!.__vector_len(this.bb_pos + offset)) : null;
}

static startSubscribe(builder:flatbuffers.Builder) {
  builder.startObject(2);
}

static addKey(builder:flatbuffers.Builder, keyOffset:flatbuffers.Offset) {
  builder.addFieldOffset(0, keyOffset, 0);
}

static addSummary(builder:flatbuffers.Builder, summaryOffset:flatbuffers.Offset) {
  builder.addFieldOffset(1, summaryOffset, 0);
}

static createSummaryVector(builder:flatbuffers.Builder, data:number[]|Uint8Array):flatbuffers.Offset {
  builder.startVector(1, data.length, 1);
  for (let i = data.length - 1; i >= 0; i--) {
    builder.addInt8(data[i]!);
  }
  return builder.endVector();
}

static startSummaryVector(builder:flatbuffers.Builder, numElems:number) {
  builder.startVector(1, numElems, 1);
}

static endSubscribe(builder:flatbuffers.Builder):flatbuffers.Offset {
  const offset = builder.endObject();
  builder.requiredField(offset, 4) // key
  return offset;
}

static createSubscribe(builder:flatbuffers.Builder, keyOffset:flatbuffers.Offset, summaryOffset:flatbuffers.Offset):flatbuffers.Offset {
  Subscribe.startSubscribe(builder);
  Subscribe.addKey(builder, keyOffset);
  Subscribe.addSummary(builder, summaryOffset);
  return Subscribe.endSubscribe(builder);
}

unpack(): SubscribeT {
  return new SubscribeT(
    (this.key() !== null ? this.key()!.unpack() : null),
    this.bb!.createScalarList<number>(this.summary.bind(this), this.summaryLength())
  );
}


unpackTo(_o: SubscribeT): void {
  _o.key = (this.key() !== null ? this.key()!.unpack() : null);
  _o.summary = this.bb!.createScalarList<number>(this.summary.bind(this), this.summaryLength());
}
}

export class SubscribeT implements flatbuffers.IGeneratedObject {
constructor(
  public key: ContractKeyT|null = null,
  public summary: (number)[] = []
){}


pack(builder:flatbuffers.Builder): flatbuffers.Offset {
  const key = (this.key !== null ? this.key!.pack(builder) : 0);
  const summary = Subscribe.createSummaryVector(builder, this.summary);

  return Subscribe.createSubscribe(builder,
    key,
    summary
  );
}
}
