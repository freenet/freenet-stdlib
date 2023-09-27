// automatically generated by the FlatBuffers compiler, do not modify

import * as flatbuffers from 'flatbuffers';

import { DelegateCode, DelegateCodeT } from '../client-request/delegate-code.js';
import { DelegateKey, DelegateKeyT } from '../client-request/delegate-key.js';


export class WasmDelegateV1 implements flatbuffers.IUnpackableObject<WasmDelegateV1T> {
  bb: flatbuffers.ByteBuffer|null = null;
  bb_pos = 0;
  __init(i:number, bb:flatbuffers.ByteBuffer):WasmDelegateV1 {
  this.bb_pos = i;
  this.bb = bb;
  return this;
}

static getRootAsWasmDelegateV1(bb:flatbuffers.ByteBuffer, obj?:WasmDelegateV1):WasmDelegateV1 {
  return (obj || new WasmDelegateV1()).__init(bb.readInt32(bb.position()) + bb.position(), bb);
}

static getSizePrefixedRootAsWasmDelegateV1(bb:flatbuffers.ByteBuffer, obj?:WasmDelegateV1):WasmDelegateV1 {
  bb.setPosition(bb.position() + flatbuffers.SIZE_PREFIX_LENGTH);
  return (obj || new WasmDelegateV1()).__init(bb.readInt32(bb.position()) + bb.position(), bb);
}

parameters(index: number):number|null {
  const offset = this.bb!.__offset(this.bb_pos, 4);
  return offset ? this.bb!.readUint8(this.bb!.__vector(this.bb_pos + offset) + index) : 0;
}

parametersLength():number {
  const offset = this.bb!.__offset(this.bb_pos, 4);
  return offset ? this.bb!.__vector_len(this.bb_pos + offset) : 0;
}

parametersArray():Uint8Array|null {
  const offset = this.bb!.__offset(this.bb_pos, 4);
  return offset ? new Uint8Array(this.bb!.bytes().buffer, this.bb!.bytes().byteOffset + this.bb!.__vector(this.bb_pos + offset), this.bb!.__vector_len(this.bb_pos + offset)) : null;
}

data(obj?:DelegateCode):DelegateCode|null {
  const offset = this.bb!.__offset(this.bb_pos, 6);
  return offset ? (obj || new DelegateCode()).__init(this.bb!.__indirect(this.bb_pos + offset), this.bb!) : null;
}

key(obj?:DelegateKey):DelegateKey|null {
  const offset = this.bb!.__offset(this.bb_pos, 8);
  return offset ? (obj || new DelegateKey()).__init(this.bb!.__indirect(this.bb_pos + offset), this.bb!) : null;
}

static startWasmDelegateV1(builder:flatbuffers.Builder) {
  builder.startObject(3);
}

static addParameters(builder:flatbuffers.Builder, parametersOffset:flatbuffers.Offset) {
  builder.addFieldOffset(0, parametersOffset, 0);
}

static createParametersVector(builder:flatbuffers.Builder, data:number[]|Uint8Array):flatbuffers.Offset {
  builder.startVector(1, data.length, 1);
  for (let i = data.length - 1; i >= 0; i--) {
    builder.addInt8(data[i]!);
  }
  return builder.endVector();
}

static startParametersVector(builder:flatbuffers.Builder, numElems:number) {
  builder.startVector(1, numElems, 1);
}

static addData(builder:flatbuffers.Builder, dataOffset:flatbuffers.Offset) {
  builder.addFieldOffset(1, dataOffset, 0);
}

static addKey(builder:flatbuffers.Builder, keyOffset:flatbuffers.Offset) {
  builder.addFieldOffset(2, keyOffset, 0);
}

static endWasmDelegateV1(builder:flatbuffers.Builder):flatbuffers.Offset {
  const offset = builder.endObject();
  builder.requiredField(offset, 4) // parameters
  builder.requiredField(offset, 6) // data
  builder.requiredField(offset, 8) // key
  return offset;
}


unpack(): WasmDelegateV1T {
  return new WasmDelegateV1T(
    this.bb!.createScalarList<number>(this.parameters.bind(this), this.parametersLength()),
    (this.data() !== null ? this.data()!.unpack() : null),
    (this.key() !== null ? this.key()!.unpack() : null)
  );
}


unpackTo(_o: WasmDelegateV1T): void {
  _o.parameters = this.bb!.createScalarList<number>(this.parameters.bind(this), this.parametersLength());
  _o.data = (this.data() !== null ? this.data()!.unpack() : null);
  _o.key = (this.key() !== null ? this.key()!.unpack() : null);
}
}

export class WasmDelegateV1T implements flatbuffers.IGeneratedObject {
constructor(
  public parameters: (number)[] = [],
  public data: DelegateCodeT|null = null,
  public key: DelegateKeyT|null = null
){}


pack(builder:flatbuffers.Builder): flatbuffers.Offset {
  const parameters = WasmDelegateV1.createParametersVector(builder, this.parameters);
  const data = (this.data !== null ? this.data!.pack(builder) : 0);
  const key = (this.key !== null ? this.key!.pack(builder) : 0);

  WasmDelegateV1.startWasmDelegateV1(builder);
  WasmDelegateV1.addParameters(builder, parameters);
  WasmDelegateV1.addData(builder, data);
  WasmDelegateV1.addKey(builder, key);

  return WasmDelegateV1.endWasmDelegateV1(builder);
}
}
