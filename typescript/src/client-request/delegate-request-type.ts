// automatically generated by the FlatBuffers compiler, do not modify

/* eslint-disable @typescript-eslint/no-unused-vars, @typescript-eslint/no-explicit-any, @typescript-eslint/no-non-null-assertion */

import { ApplicationMessages, ApplicationMessagesT } from '../client-request/application-messages.js';
import { GetSecretRequestType, GetSecretRequestTypeT } from '../client-request/get-secret-request-type.js';
import { RegisterDelegate, RegisterDelegateT } from '../client-request/register-delegate.js';
import { UnregisterDelegate, UnregisterDelegateT } from '../client-request/unregister-delegate.js';


export enum DelegateRequestType {
  NONE = 0,
  ApplicationMessages = 1,
  GetSecretRequestType = 2,
  RegisterDelegate = 3,
  UnregisterDelegate = 4
}

export function unionToDelegateRequestType(
  type: DelegateRequestType,
  accessor: (obj:ApplicationMessages|GetSecretRequestType|RegisterDelegate|UnregisterDelegate) => ApplicationMessages|GetSecretRequestType|RegisterDelegate|UnregisterDelegate|null
): ApplicationMessages|GetSecretRequestType|RegisterDelegate|UnregisterDelegate|null {
  switch(DelegateRequestType[type]) {
    case 'NONE': return null; 
    case 'ApplicationMessages': return accessor(new ApplicationMessages())! as ApplicationMessages;
    case 'GetSecretRequestType': return accessor(new GetSecretRequestType())! as GetSecretRequestType;
    case 'RegisterDelegate': return accessor(new RegisterDelegate())! as RegisterDelegate;
    case 'UnregisterDelegate': return accessor(new UnregisterDelegate())! as UnregisterDelegate;
    default: return null;
  }
}

export function unionListToDelegateRequestType(
  type: DelegateRequestType, 
  accessor: (index: number, obj:ApplicationMessages|GetSecretRequestType|RegisterDelegate|UnregisterDelegate) => ApplicationMessages|GetSecretRequestType|RegisterDelegate|UnregisterDelegate|null, 
  index: number
): ApplicationMessages|GetSecretRequestType|RegisterDelegate|UnregisterDelegate|null {
  switch(DelegateRequestType[type]) {
    case 'NONE': return null; 
    case 'ApplicationMessages': return accessor(index, new ApplicationMessages())! as ApplicationMessages;
    case 'GetSecretRequestType': return accessor(index, new GetSecretRequestType())! as GetSecretRequestType;
    case 'RegisterDelegate': return accessor(index, new RegisterDelegate())! as RegisterDelegate;
    case 'UnregisterDelegate': return accessor(index, new UnregisterDelegate())! as UnregisterDelegate;
    default: return null;
  }
}
