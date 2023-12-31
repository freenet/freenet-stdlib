// automatically generated by the FlatBuffers compiler, do not modify

/* eslint-disable @typescript-eslint/no-unused-vars, @typescript-eslint/no-explicit-any, @typescript-eslint/no-non-null-assertion */

import * as flatbuffers from 'flatbuffers';

import { SecretsId, SecretsIdT } from '../common/secrets-id.js';


export class GetSecretRequest implements flatbuffers.IUnpackableObject<GetSecretRequestT> {
  bb: flatbuffers.ByteBuffer|null = null;
  bb_pos = 0;
  __init(i:number, bb:flatbuffers.ByteBuffer):GetSecretRequest {
  this.bb_pos = i;
  this.bb = bb;
  return this;
}

static getRootAsGetSecretRequest(bb:flatbuffers.ByteBuffer, obj?:GetSecretRequest):GetSecretRequest {
  return (obj || new GetSecretRequest()).__init(bb.readInt32(bb.position()) + bb.position(), bb);
}

static getSizePrefixedRootAsGetSecretRequest(bb:flatbuffers.ByteBuffer, obj?:GetSecretRequest):GetSecretRequest {
  bb.setPosition(bb.position() + flatbuffers.SIZE_PREFIX_LENGTH);
  return (obj || new GetSecretRequest()).__init(bb.readInt32(bb.position()) + bb.position(), bb);
}

key(obj?:SecretsId):SecretsId|null {
  const offset = this.bb!.__offset(this.bb_pos, 4);
  return offset ? (obj || new SecretsId()).__init(this.bb!.__indirect(this.bb_pos + offset), this.bb!) : null;
}

delegateContext(index: number):number|null {
  const offset = this.bb!.__offset(this.bb_pos, 6);
  return offset ? this.bb!.readUint8(this.bb!.__vector(this.bb_pos + offset) + index) : 0;
}

delegateContextLength():number {
  const offset = this.bb!.__offset(this.bb_pos, 6);
  return offset ? this.bb!.__vector_len(this.bb_pos + offset) : 0;
}

delegateContextArray():Uint8Array|null {
  const offset = this.bb!.__offset(this.bb_pos, 6);
  return offset ? new Uint8Array(this.bb!.bytes().buffer, this.bb!.bytes().byteOffset + this.bb!.__vector(this.bb_pos + offset), this.bb!.__vector_len(this.bb_pos + offset)) : null;
}

processed():boolean {
  const offset = this.bb!.__offset(this.bb_pos, 8);
  return offset ? !!this.bb!.readInt8(this.bb_pos + offset) : false;
}

static startGetSecretRequest(builder:flatbuffers.Builder) {
  builder.startObject(3);
}

static addKey(builder:flatbuffers.Builder, keyOffset:flatbuffers.Offset) {
  builder.addFieldOffset(0, keyOffset, 0);
}

static addDelegateContext(builder:flatbuffers.Builder, delegateContextOffset:flatbuffers.Offset) {
  builder.addFieldOffset(1, delegateContextOffset, 0);
}

static createDelegateContextVector(builder:flatbuffers.Builder, data:number[]|Uint8Array):flatbuffers.Offset {
  builder.startVector(1, data.length, 1);
  for (let i = data.length - 1; i >= 0; i--) {
    builder.addInt8(data[i]!);
  }
  return builder.endVector();
}

static startDelegateContextVector(builder:flatbuffers.Builder, numElems:number) {
  builder.startVector(1, numElems, 1);
}

static addProcessed(builder:flatbuffers.Builder, processed:boolean) {
  builder.addFieldInt8(2, +processed, +false);
}

static endGetSecretRequest(builder:flatbuffers.Builder):flatbuffers.Offset {
  const offset = builder.endObject();
  builder.requiredField(offset, 4) // key
  builder.requiredField(offset, 6) // delegate_context
  return offset;
}

static createGetSecretRequest(builder:flatbuffers.Builder, keyOffset:flatbuffers.Offset, delegateContextOffset:flatbuffers.Offset, processed:boolean):flatbuffers.Offset {
  GetSecretRequest.startGetSecretRequest(builder);
  GetSecretRequest.addKey(builder, keyOffset);
  GetSecretRequest.addDelegateContext(builder, delegateContextOffset);
  GetSecretRequest.addProcessed(builder, processed);
  return GetSecretRequest.endGetSecretRequest(builder);
}

unpack(): GetSecretRequestT {
  return new GetSecretRequestT(
    (this.key() !== null ? this.key()!.unpack() : null),
    this.bb!.createScalarList<number>(this.delegateContext.bind(this), this.delegateContextLength()),
    this.processed()
  );
}


unpackTo(_o: GetSecretRequestT): void {
  _o.key = (this.key() !== null ? this.key()!.unpack() : null);
  _o.delegateContext = this.bb!.createScalarList<number>(this.delegateContext.bind(this), this.delegateContextLength());
  _o.processed = this.processed();
}
}

export class GetSecretRequestT implements flatbuffers.IGeneratedObject {
constructor(
  public key: SecretsIdT|null = null,
  public delegateContext: (number)[] = [],
  public processed: boolean = false
){}


pack(builder:flatbuffers.Builder): flatbuffers.Offset {
  const key = (this.key !== null ? this.key!.pack(builder) : 0);
  const delegateContext = GetSecretRequest.createDelegateContextVector(builder, this.delegateContext);

  return GetSecretRequest.createGetSecretRequest(builder,
    key,
    delegateContext,
    this.processed
  );
}
}
