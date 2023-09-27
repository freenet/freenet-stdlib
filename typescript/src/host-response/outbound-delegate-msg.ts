// automatically generated by the FlatBuffers compiler, do not modify

import * as flatbuffers from 'flatbuffers';

import { ApplicationMessage, ApplicationMessageT } from '../common/application-message.js';
import { GetSecretRequest, GetSecretRequestT } from '../common/get-secret-request.js';
import { GetSecretResponse, GetSecretResponseT } from '../common/get-secret-response.js';
import { ContextUpdated, ContextUpdatedT } from '../host-response/context-updated.js';
import { OutboundDelegateMsgType, unionToOutboundDelegateMsgType, unionListToOutboundDelegateMsgType } from '../host-response/outbound-delegate-msg-type.js';
import { RequestUserInput, RequestUserInputT } from '../host-response/request-user-input.js';
import { SetSecretRequest, SetSecretRequestT } from '../host-response/set-secret-request.js';


export class OutboundDelegateMsg implements flatbuffers.IUnpackableObject<OutboundDelegateMsgT> {
  bb: flatbuffers.ByteBuffer|null = null;
  bb_pos = 0;
  __init(i:number, bb:flatbuffers.ByteBuffer):OutboundDelegateMsg {
  this.bb_pos = i;
  this.bb = bb;
  return this;
}

static getRootAsOutboundDelegateMsg(bb:flatbuffers.ByteBuffer, obj?:OutboundDelegateMsg):OutboundDelegateMsg {
  return (obj || new OutboundDelegateMsg()).__init(bb.readInt32(bb.position()) + bb.position(), bb);
}

static getSizePrefixedRootAsOutboundDelegateMsg(bb:flatbuffers.ByteBuffer, obj?:OutboundDelegateMsg):OutboundDelegateMsg {
  bb.setPosition(bb.position() + flatbuffers.SIZE_PREFIX_LENGTH);
  return (obj || new OutboundDelegateMsg()).__init(bb.readInt32(bb.position()) + bb.position(), bb);
}

inboundType():OutboundDelegateMsgType {
  const offset = this.bb!.__offset(this.bb_pos, 4);
  return offset ? this.bb!.readUint8(this.bb_pos + offset) : OutboundDelegateMsgType.NONE;
}

inbound<T extends flatbuffers.Table>(obj:any):any|null {
  const offset = this.bb!.__offset(this.bb_pos, 6);
  return offset ? this.bb!.__union(obj, this.bb_pos + offset) : null;
}

static startOutboundDelegateMsg(builder:flatbuffers.Builder) {
  builder.startObject(2);
}

static addInboundType(builder:flatbuffers.Builder, inboundType:OutboundDelegateMsgType) {
  builder.addFieldInt8(0, inboundType, OutboundDelegateMsgType.NONE);
}

static addInbound(builder:flatbuffers.Builder, inboundOffset:flatbuffers.Offset) {
  builder.addFieldOffset(1, inboundOffset, 0);
}

static endOutboundDelegateMsg(builder:flatbuffers.Builder):flatbuffers.Offset {
  const offset = builder.endObject();
  builder.requiredField(offset, 6) // inbound
  return offset;
}

static createOutboundDelegateMsg(builder:flatbuffers.Builder, inboundType:OutboundDelegateMsgType, inboundOffset:flatbuffers.Offset):flatbuffers.Offset {
  OutboundDelegateMsg.startOutboundDelegateMsg(builder);
  OutboundDelegateMsg.addInboundType(builder, inboundType);
  OutboundDelegateMsg.addInbound(builder, inboundOffset);
  return OutboundDelegateMsg.endOutboundDelegateMsg(builder);
}

unpack(): OutboundDelegateMsgT {
  return new OutboundDelegateMsgT(
    this.inboundType(),
    (() => {
      const temp = unionToOutboundDelegateMsgType(this.inboundType(), this.inbound.bind(this));
      if(temp === null) { return null; }
      return temp.unpack()
  })()
  );
}


unpackTo(_o: OutboundDelegateMsgT): void {
  _o.inboundType = this.inboundType();
  _o.inbound = (() => {
      const temp = unionToOutboundDelegateMsgType(this.inboundType(), this.inbound.bind(this));
      if(temp === null) { return null; }
      return temp.unpack()
  })();
}
}

export class OutboundDelegateMsgT implements flatbuffers.IGeneratedObject {
constructor(
  public inboundType: OutboundDelegateMsgType = OutboundDelegateMsgType.NONE,
  public inbound: ApplicationMessageT|ContextUpdatedT|GetSecretRequestT|GetSecretResponseT|RequestUserInputT|SetSecretRequestT|null = null
){}


pack(builder:flatbuffers.Builder): flatbuffers.Offset {
  const inbound = builder.createObjectOffset(this.inbound);

  return OutboundDelegateMsg.createOutboundDelegateMsg(builder,
    this.inboundType,
    inbound
  );
}
}
