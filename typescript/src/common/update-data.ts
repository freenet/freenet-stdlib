// automatically generated by the FlatBuffers compiler, do not modify

import * as flatbuffers from 'flatbuffers';

import { DeltaUpdate, DeltaUpdateT } from '../common/delta-update.js';
import { RelatedDeltaUpdate, RelatedDeltaUpdateT } from '../common/related-delta-update.js';
import { RelatedStateAndDeltaUpdate, RelatedStateAndDeltaUpdateT } from '../common/related-state-and-delta-update.js';
import { RelatedStateUpdate, RelatedStateUpdateT } from '../common/related-state-update.js';
import { StateAndDeltaUpdate, StateAndDeltaUpdateT } from '../common/state-and-delta-update.js';
import { StateUpdate, StateUpdateT } from '../common/state-update.js';
import { UpdateDataType, unionToUpdateDataType, unionListToUpdateDataType } from '../common/update-data-type.js';


export class UpdateData implements flatbuffers.IUnpackableObject<UpdateDataT> {
  bb: flatbuffers.ByteBuffer|null = null;
  bb_pos = 0;
  __init(i:number, bb:flatbuffers.ByteBuffer):UpdateData {
  this.bb_pos = i;
  this.bb = bb;
  return this;
}

static getRootAsUpdateData(bb:flatbuffers.ByteBuffer, obj?:UpdateData):UpdateData {
  return (obj || new UpdateData()).__init(bb.readInt32(bb.position()) + bb.position(), bb);
}

static getSizePrefixedRootAsUpdateData(bb:flatbuffers.ByteBuffer, obj?:UpdateData):UpdateData {
  bb.setPosition(bb.position() + flatbuffers.SIZE_PREFIX_LENGTH);
  return (obj || new UpdateData()).__init(bb.readInt32(bb.position()) + bb.position(), bb);
}

updateDataType():UpdateDataType {
  const offset = this.bb!.__offset(this.bb_pos, 4);
  return offset ? this.bb!.readUint8(this.bb_pos + offset) : UpdateDataType.NONE;
}

updateData<T extends flatbuffers.Table>(obj:any):any|null {
  const offset = this.bb!.__offset(this.bb_pos, 6);
  return offset ? this.bb!.__union(obj, this.bb_pos + offset) : null;
}

static startUpdateData(builder:flatbuffers.Builder) {
  builder.startObject(2);
}

static addUpdateDataType(builder:flatbuffers.Builder, updateDataType:UpdateDataType) {
  builder.addFieldInt8(0, updateDataType, UpdateDataType.NONE);
}

static addUpdateData(builder:flatbuffers.Builder, updateDataOffset:flatbuffers.Offset) {
  builder.addFieldOffset(1, updateDataOffset, 0);
}

static endUpdateData(builder:flatbuffers.Builder):flatbuffers.Offset {
  const offset = builder.endObject();
  builder.requiredField(offset, 6) // update_data
  return offset;
}

static createUpdateData(builder:flatbuffers.Builder, updateDataType:UpdateDataType, updateDataOffset:flatbuffers.Offset):flatbuffers.Offset {
  UpdateData.startUpdateData(builder);
  UpdateData.addUpdateDataType(builder, updateDataType);
  UpdateData.addUpdateData(builder, updateDataOffset);
  return UpdateData.endUpdateData(builder);
}

unpack(): UpdateDataT {
  return new UpdateDataT(
    this.updateDataType(),
    (() => {
      const temp = unionToUpdateDataType(this.updateDataType(), this.updateData.bind(this));
      if(temp === null) { return null; }
      return temp.unpack()
  })()
  );
}


unpackTo(_o: UpdateDataT): void {
  _o.updateDataType = this.updateDataType();
  _o.updateData = (() => {
      const temp = unionToUpdateDataType(this.updateDataType(), this.updateData.bind(this));
      if(temp === null) { return null; }
      return temp.unpack()
  })();
}
}

export class UpdateDataT implements flatbuffers.IGeneratedObject {
constructor(
  public updateDataType: UpdateDataType = UpdateDataType.NONE,
  public updateData: DeltaUpdateT|RelatedDeltaUpdateT|RelatedStateAndDeltaUpdateT|RelatedStateUpdateT|StateAndDeltaUpdateT|StateUpdateT|null = null
){}


pack(builder:flatbuffers.Builder): flatbuffers.Offset {
  const updateData = builder.createObjectOffset(this.updateData);

  return UpdateData.createUpdateData(builder,
    this.updateDataType,
    updateData
  );
}
}
