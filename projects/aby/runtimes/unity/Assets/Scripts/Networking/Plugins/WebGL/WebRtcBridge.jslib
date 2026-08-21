// The WebGL half of the WebRTC transport split — com.unity.webrtc (native libwebrtc) can't run
// here, so this wraps the browser's own RTCPeerConnection directly. See
// WebGLWebRtcTransport.cs for the C# side these functions are called from, and for how incoming
// events (ice candidates, data channel messages) get called back into C# via SendMessage.
//
// One RTCPeerConnection per peer, keyed by peerId, same shape as NativeWebRtcTransport's
// `connections` dictionary.

mergeInto(LibraryManager.library, {

  WebRtcBridge_Init: function (gameObjectNamePtr) {
    window.__abyWebRtc = window.__abyWebRtc || {
      gameObjectName: UTF8ToString(gameObjectNamePtr),
      connections: {},
      channels: {},
    };
  },

  WebRtcBridge_CreateConnection: function (peerIdPtr, isOffererValue) {
    var peerId = UTF8ToString(peerIdPtr);
    var isOfferer = !!isOffererValue;
    var state = window.__abyWebRtc;

    var pc = new RTCPeerConnection({ iceServers: [{ urls: "stun:stun.l.google.com:19302" }] });
    state.connections[peerId] = pc;

    pc.onicecandidate = function (event) {
      if (!event.candidate) return;
      var bundle = JSON.stringify({
        candidate: event.candidate.candidate,
        sdpMid: event.candidate.sdpMid,
        sdpMLineIndex: event.candidate.sdpMLineIndex,
      });
      unityInstance.SendMessage(state.gameObjectName, 'OnJsIceCandidate', peerId + '|' + bundle);
    };

    var wireChannel = function (channel) {
      channel.binaryType = 'arraybuffer';
      state.channels[peerId] = channel;

      channel.onopen = function () {
        unityInstance.SendMessage(state.gameObjectName, 'OnJsChannelOpen', peerId);
      };

      channel.onmessage = function (event) {
        // Base64-encode: SendMessage only carries strings, and the data may be binary.
        var bytes = new Uint8Array(event.data);
        var binary = '';
        for (var i = 0; i < bytes.byteLength; i++) binary += String.fromCharCode(bytes[i]);
        unityInstance.SendMessage(state.gameObjectName, 'OnJsDataReceived', peerId + '|' + btoa(binary));
      };
    };

    if (isOfferer) {
      wireChannel(pc.createDataChannel('data'));
    } else {
      pc.ondatachannel = function (event) { wireChannel(event.channel); };
    }
  },

  WebRtcBridge_CreateOffer: function (peerIdPtr) {
    var peerId = UTF8ToString(peerIdPtr);
    var state = window.__abyWebRtc;
    var pc = state.connections[peerId];

    pc.createOffer()
      .then(function (offer) { return pc.setLocalDescription(offer); })
      .then(function () {
        unityInstance.SendMessage(state.gameObjectName, 'OnJsLocalDescription', peerId + '|offer|' + pc.localDescription.sdp);
      });
  },

  WebRtcBridge_SetRemoteDescription: function (peerIdPtr, typePtr, sdpPtr) {
    var peerId = UTF8ToString(peerIdPtr);
    var type = UTF8ToString(typePtr);
    var sdp = UTF8ToString(sdpPtr);
    var state = window.__abyWebRtc;
    var pc = state.connections[peerId];

    pc.setRemoteDescription({ type: type, sdp: sdp }).then(function () {
      if (type === 'offer') {
        pc.createAnswer()
          .then(function (answer) { return pc.setLocalDescription(answer); })
          .then(function () {
            unityInstance.SendMessage(state.gameObjectName, 'OnJsLocalDescription', peerId + '|answer|' + pc.localDescription.sdp);
          });
      }
    });
  },

  WebRtcBridge_AddIceCandidate: function (peerIdPtr, candidateJsonPtr) {
    var peerId = UTF8ToString(peerIdPtr);
    var bundle = JSON.parse(UTF8ToString(candidateJsonPtr));
    var state = window.__abyWebRtc;
    var pc = state.connections[peerId];

    pc.addIceCandidate(new RTCIceCandidate(bundle));
  },

  WebRtcBridge_Send: function (peerIdPtr, base64Ptr) {
    var peerId = UTF8ToString(peerIdPtr);
    var base64 = UTF8ToString(base64Ptr);
    var state = window.__abyWebRtc;
    var channel = state.channels[peerId];
    if (!channel || channel.readyState !== 'open') return;

    var binary = atob(base64);
    var bytes = new Uint8Array(binary.length);
    for (var i = 0; i < binary.length; i++) bytes[i] = binary.charCodeAt(i);
    channel.send(bytes);
  },

  WebRtcBridge_Close: function (peerIdPtr) {
    var peerId = UTF8ToString(peerIdPtr);
    var state = window.__abyWebRtc;
    if (state.channels[peerId]) { state.channels[peerId].close(); delete state.channels[peerId]; }
    if (state.connections[peerId]) { state.connections[peerId].close(); delete state.connections[peerId]; }
  },

});
