# DataVir Messages

All messages are sent through CWTs (COSE Web Tokens) and the following claims are mandatory:

  * `iat`: Issued At.
  * `iss`: Issuer, in the format `{user-uuid} via {application-uuid-or-url}`, e.g. `526AAD16-3B4B-4156-BE7F-68ED5D14D529 via 4B3232A2-7DAB-4FE9-A40A-717FF7FF50A2`, `526AAD16-3B4B-4156-BE7F-68ED5D14D529 via myapp.example.com`.

User and authentication stuff is going to be handled latter. (we will assume for now that there is always a single user with a single key)

#### Message

```cddl
final-req = get-time-msg / add-file-msg / ...
final-msg = final-req / final-rpl / final-ntc
# req = request
# rpl = reply
# ntc = notice
```

#### Time

```cddl
getTimeReq = {
	msgType: "getTimeReq"
}
```

```cddl
getTimeRpl = {
	msgType: "getTimeRpl"
	currentTime: time
}
```

#### Volumes

```cddl
listVolumesReq = {
	msgType: "listVolumesReq"
}
```

```cddl
listVolumesRpl = {
	msgType: "listVolumesRpl"
	volumes: [* volumeInfo]
}
```

```cddl
volumeInfo = {
	uuid: uuid
	title: tstr
	name: tstr
	isReal: bool
	uid2name: { * uint16 => tstr }
	gid2name: { * uint16 => tstr }
}
```

#### Nodes

```cddl
nodeInfoReq = {
	msgType: "nodeInfoReq"
	nodesOrPaths: [ * uuid / tstr ]
	volume: uuid ?
}
```

```cddl
nodeInfoRpl = {
	msgType: "nodeInfoRpl"
	nodes: [ * nodeInfo / error ]
	paths2uuid: { * tstr => uuid }
}
```

```cddl
nodeInfo = {
	uuid: uuid
	name: tstr
	title: tstr
	description: tstr
	parents: [* uuid]
	content: contentRef
	thumbnail: bstr ?
	unixPerm: unixPerm ?
	xattrs: {* tstr => xattrVal}
	created: time
	changed: time
	volume: uuid
	inTrash: bool
	trashedBy: uuid ?
	trashedWhen: time ?
}

unixPerm = {
	mode: uint16
	uid: uint16
	gid: uint16
}

contentRef = {
	file-kind: "empty" / "regular" / "symbolic-link" / "hard-link" / "socket"
	copyOnWrite: bool
	stream: uuid
}

xattrVal = {
	format: tstr,
	value: bstr
}
```

#### Streams

```cddl
streamHashReq = {
	msgType: "streamHashReq"
	alg: hashAlg
	nodesOrPaths: [ * uuid / tstr ]
	volume: uuid ?
}
```

```cddl
streamHashRpl = {
	alg: hashAlg
	values: { * uuid => bstr }
	paths2uuid: { * tstr => uuid }
}
```