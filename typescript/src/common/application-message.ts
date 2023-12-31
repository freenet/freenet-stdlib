// automatically generated by the FlatBuffers compiler, do not modify

import * as flatbuffers from 'flatbuffers';

import { ContractInstanceId, ContractInstanceIdT } from '../common/contract-instance-id.js';


export class ApplicationMessage implements flatbuffers.IUnpackableObject<ApplicationMessageT> {
  bb: flatbuffers.ByteBuffer|null = null;
  bb_pos = 0;
  __init(i:number, bb:flatbuffers.ByteBuffer):ApplicationMessage {
  this.bb_pos = i;
  this.bb = bb;
  return this;
}

static getRootAsApplicationMessage(bb:flatbuffers.ByteBuffer, obj?:ApplicationMessage):ApplicationMessage {
  return (obj || new ApplicationMessage()).__init(bb.readInt32(bb.position()) + bb.position(), bb);
}

static getSizePrefixedRootAsApplicationMessage(bb:flatbuffers.ByteBuffer, obj?:ApplicationMessage):ApplicationMessage {
  bb.setPosition(bb.position() + flatbuffers.SIZE_PREFIX_LENGTH);
  return (obj || new ApplicationMessage()).__init(bb.readInt32(bb.position()) + bb.position(), bb);
}

app(obj?:ContractInstanceId):ContractInstanceId|null {
  const offset = this.bb!.__offset(this.bb_pos, 4);
  return offset ? (obj || new ContractInstanceId()).__init(this.bb!.__indirect(this.bb_pos + offset), this.bb!) : null;
}

payload(index: number):number|null {
  const offset = this.bb!.__offset(this.bb_pos, 6);
  return offset ? this.bb!.readUint8(this.bb!.__vector(this.bb_pos + offset) + index) : 0;
}

payloadLength():number {
  const offset = this.bb!.__offset(this.bb_pos, 6);
  return offset ? this.bb!.__vector_len(this.bb_pos + offset) : 0;
}

payloadArray():Uint8Array|null {
  const offset = this.bb!.__offset(this.bb_pos, 6);
  return offset ? new Uint8Array(this.bb!.bytes().buffer, this.bb!.bytes().byteOffset + this.bb!.__vector(this.bb_pos + offset), this.bb!.__vector_len(this.bb_pos + offset)) : null;
}

context(index: number):number|null {
  const offset = this.bb!.__offset(this.bb_pos, 8);
  return offset ? this.bb!.readUint8(this.bb!.__vector(this.bb_pos + offset) + index) : 0;
}

contextLength():number {
  const offset = this.bb!.__offset(this.bb_pos, 8);
  return offset ? this.bb!.__vector_len(this.bb_pos + offset) : 0;
}

contextArray():Uint8Array|null {
  const offset = this.bb!.__offset(this.bb_pos, 8);
  return offset ? new Uint8Array(this.bb!.bytes().buffer, this.bb!.bytes().byteOffset + this.bb!.__vector(this.bb_pos + offset), this.bb!.__vector_len(this.bb_pos + offset)) : null;
}

processed():boolean {
  const offset = this.bb!.__offset(this.bb_pos, 10);
  return offset ? !!this.bb!.readInt8(this.bb_pos + offset) : false;
}

static startApplicationMessage(builder:flatbuffers.Builder) {
  builder.startObject(4);
}

static addApp(builder:flatbuffers.Builder, appOffset:flatbuffers.Offset) {
  builder.addFieldOffset(0, appOffset, 0);
}

static addPayload(builder:flatbuffers.Builder, payloadOffset:flatbuffers.Offset) {
  builder.addFieldOffset(1, payloadOffset, 0);
}

static createPayloadVector(builder:flatbuffers.Builder, data:number[]|Uint8Array):flatbuffers.Offset {
  builder.startVector(1, data.length, 1);
  for (let i = data.length - 1; i >= 0; i--) {
    builder.addInt8(data[i]!);
  }
  return builder.endVector();
}

static startPayloadVector(builder:flatbuffers.Builder, numElems:number) {
  builder.startVector(1, numElems, 1);
}

static addContext(builder:flatbuffers.Builder, contextOffset:flatbuffers.Offset) {
  builder.addFieldOffset(2, contextOffset, 0);
}

static createContextVector(builder:flatbuffers.Builder, data:number[]|Uint8Array):flatbuffers.Offset {
  builder.startVector(1, data.length, 1);
  for (let i = data.length - 1; i >= 0; i--) {
    builder.addInt8(data[i]!);
  }
  return builder.endVector();
}

static startContextVector(builder:flatbuffers.Builder, numElems:number) {
  builder.startVector(1, numElems, 1);
}

static addProcessed(builder:flatbuffers.Builder, processed:boolean) {
  builder.addFieldInt8(3, +processed, +false);
}

static endApplicationMessage(builder:flatbuffers.Builder):flatbuffers.Offset {
  const offset = builder.endObject();
  builder.requiredField(offset, 4) // app
  builder.requiredField(offset, 6) // payload
  builder.requiredField(offset, 8) // context
  return offset;
}

static createApplicationMessage(builder:flatbuffers.Builder, appOffset:flatbuffers.Offset, payloadOffset:flatbuffers.Offset, contextOffset:flatbuffers.Offset, processed:boolean):flatbuffers.Offset {
  ApplicationMessage.startApplicationMessage(builder);
  ApplicationMessage.addApp(builder, appOffset);
  ApplicationMessage.addPayload(builder, payloadOffset);
  ApplicationMessage.addContext(builder, contextOffset);
  ApplicationMessage.addProcessed(builder, processed);
  return ApplicationMessage.endApplicationMessage(builder);
}

unpack(): ApplicationMessageT {
  return new ApplicationMessageT(
    (this.app() !== null ? this.app()!.unpack() : null),
    this.bb!.createScalarList<number>(this.payload.bind(this), this.payloadLength()),
    this.bb!.createScalarList<number>(this.context.bind(this), this.contextLength()),
    this.processed()
  );
}


unpackTo(_o: ApplicationMessageT): void {
  _o.app = (this.app() !== null ? this.app()!.unpack() : null);
  _o.payload = this.bb!.createScalarList<number>(this.payload.bind(this), this.payloadLength());
  _o.context = this.bb!.createScalarList<number>(this.context.bind(this), this.contextLength());
  _o.processed = this.processed();
}
}

export class ApplicationMessageT implements flatbuffers.IGeneratedObject {
constructor(
  public app: ContractInstanceIdT|null = null,
  public payload: (number)[] = [],
  public context: (number)[] = [],
  public processed: boolean = false
){}


pack(builder:flatbuffers.Builder): flatbuffers.Offset {
  const app = (this.app !== null ? this.app!.pack(builder) : 0);
  const payload = ApplicationMessage.createPayloadVector(builder, this.payload);
  const context = ApplicationMessage.createContextVector(builder, this.context);

  return ApplicationMessage.createApplicationMessage(builder,
    app,
    payload,
    context,
    this.processed
  );
}
}
