// automatically generated by the FlatBuffers compiler, do not modify

/* eslint-disable @typescript-eslint/no-unused-vars, @typescript-eslint/no-explicit-any, @typescript-eslint/no-non-null-assertion */

import * as flatbuffers from 'flatbuffers';

import { ContractKey, ContractKeyT } from '../common/contract-key.js';
import { UpdateData, UpdateDataT } from '../common/update-data.js';


export class Update implements flatbuffers.IUnpackableObject<UpdateT> {
  bb: flatbuffers.ByteBuffer|null = null;
  bb_pos = 0;
  __init(i:number, bb:flatbuffers.ByteBuffer):Update {
  this.bb_pos = i;
  this.bb = bb;
  return this;
}

static getRootAsUpdate(bb:flatbuffers.ByteBuffer, obj?:Update):Update {
  return (obj || new Update()).__init(bb.readInt32(bb.position()) + bb.position(), bb);
}

static getSizePrefixedRootAsUpdate(bb:flatbuffers.ByteBuffer, obj?:Update):Update {
  bb.setPosition(bb.position() + flatbuffers.SIZE_PREFIX_LENGTH);
  return (obj || new Update()).__init(bb.readInt32(bb.position()) + bb.position(), bb);
}

key(obj?:ContractKey):ContractKey|null {
  const offset = this.bb!.__offset(this.bb_pos, 4);
  return offset ? (obj || new ContractKey()).__init(this.bb!.__indirect(this.bb_pos + offset), this.bb!) : null;
}

data(obj?:UpdateData):UpdateData|null {
  const offset = this.bb!.__offset(this.bb_pos, 6);
  return offset ? (obj || new UpdateData()).__init(this.bb!.__indirect(this.bb_pos + offset), this.bb!) : null;
}

static startUpdate(builder:flatbuffers.Builder) {
  builder.startObject(2);
}

static addKey(builder:flatbuffers.Builder, keyOffset:flatbuffers.Offset) {
  builder.addFieldOffset(0, keyOffset, 0);
}

static addData(builder:flatbuffers.Builder, dataOffset:flatbuffers.Offset) {
  builder.addFieldOffset(1, dataOffset, 0);
}

static endUpdate(builder:flatbuffers.Builder):flatbuffers.Offset {
  const offset = builder.endObject();
  builder.requiredField(offset, 4) // key
  builder.requiredField(offset, 6) // data
  return offset;
}


unpack(): UpdateT {
  return new UpdateT(
    (this.key() !== null ? this.key()!.unpack() : null),
    (this.data() !== null ? this.data()!.unpack() : null)
  );
}


unpackTo(_o: UpdateT): void {
  _o.key = (this.key() !== null ? this.key()!.unpack() : null);
  _o.data = (this.data() !== null ? this.data()!.unpack() : null);
}
}

export class UpdateT implements flatbuffers.IGeneratedObject {
constructor(
  public key: ContractKeyT|null = null,
  public data: UpdateDataT|null = null
){}


pack(builder:flatbuffers.Builder): flatbuffers.Offset {
  const key = (this.key !== null ? this.key!.pack(builder) : 0);
  const data = (this.data !== null ? this.data!.pack(builder) : 0);

  Update.startUpdate(builder);
  Update.addKey(builder, key);
  Update.addData(builder, data);

  return Update.endUpdate(builder);
}
}
