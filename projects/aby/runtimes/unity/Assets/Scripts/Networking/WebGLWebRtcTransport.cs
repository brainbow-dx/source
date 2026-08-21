#if UNITY_WEBGL && !UNITY_EDITOR
using System;
using System.Collections.Generic;
using System.Runtime.InteropServices;
using Newtonsoft.Json;
using UnityEngine;

namespace Aby.Networking
{
    /// <summary>
    /// <see cref="IPeerTransport"/> backed by the browser's own <c>RTCPeerConnection</c> via
    /// <c>WebRtcBridge.jslib</c> — <c>com.unity.webrtc</c> can't run in WebGL builds (it wraps
    /// native libwebrtc, and browsers can't load native plugins), so this is a separate
    /// implementation of the same interface, not a variant of <see
    /// cref="NativeWebRtcTransport"/>. Only compiled into WebGL builds (the <c>#if</c> above);
    /// the Editor always uses the native backend, even when WebGL is the active build target.
    ///
    /// Incoming events arrive via <c>SendMessage</c> from the JS side, which means the methods
    /// below named <c>OnJs*</c> must stay public, keep their exact names, and live on the same
    /// GameObject this component is attached to, this is JS calling into Unity by convention,
    /// not a normal C# call path.
    /// </summary>
    public class WebGLWebRtcTransport : MonoBehaviour, IPeerTransport
    {
        [DllImport("__Internal")] static extern void WebRtcBridge_Init(string gameObjectName);
        [DllImport("__Internal")] static extern void WebRtcBridge_CreateConnection(string peerId, bool isOfferer);
        [DllImport("__Internal")] static extern void WebRtcBridge_CreateOffer(string peerId);
        [DllImport("__Internal")] static extern void WebRtcBridge_SetRemoteDescription(string peerId, string type, string sdp);
        [DllImport("__Internal")] static extern void WebRtcBridge_AddIceCandidate(string peerId, string candidateJson);
        [DllImport("__Internal")] static extern void WebRtcBridge_Send(string peerId, string base64);
        [DllImport("__Internal")] static extern void WebRtcBridge_Close(string peerId);

        SignalingClient signaling;
        readonly HashSet<string> knownPeers = new();

        public string LocalPeerId => signaling?.LocalPeerId;

        public event Action<string> PeerJoined;
        public event Action<string> PeerLeft;
        public event Action<string> PeerConnected;
        public event Action<string, byte[]> DataReceived;

        void Awake()
        {
            WebRtcBridge_Init(gameObject.name);
        }

        public void Connect(string relayUrl, string room)
        {
            signaling = new SignalingClient(relayUrl);
            signaling.Joined += existingPeers =>
            {
                foreach (var peerId in existingPeers)
                {
                    knownPeers.Add(peerId);
                    WebRtcBridge_CreateConnection(peerId, true);
                    WebRtcBridge_CreateOffer(peerId);
                }
            };
            signaling.PeerJoined += peerId => PeerJoined?.Invoke(peerId);
            signaling.PeerLeft += peerId =>
            {
                knownPeers.Remove(peerId);
                WebRtcBridge_Close(peerId);
                PeerLeft?.Invoke(peerId);
            };
            signaling.OfferReceived += (from, sdp) =>
            {
                knownPeers.Add(from);
                WebRtcBridge_CreateConnection(from, false);
                WebRtcBridge_SetRemoteDescription(from, "offer", sdp);
            };
            signaling.AnswerReceived += (from, sdp) => WebRtcBridge_SetRemoteDescription(from, "answer", sdp);
            signaling.IceCandidateReceived += (from, candidateJson) => WebRtcBridge_AddIceCandidate(from, candidateJson);

            signaling.Connect(room);
        }

        public void Disconnect()
        {
            foreach (var peerId in knownPeers)
            {
                WebRtcBridge_Close(peerId);
            }
            knownPeers.Clear();
            signaling?.Disconnect();
        }

        public void SendData(string peerId, byte[] data) => WebRtcBridge_Send(peerId, Convert.ToBase64String(data));

        void Update() => signaling?.DispatchMessageQueue();

        // Called by WebRtcBridge.jslib via SendMessage — see the class doc comment above.

        public void OnJsIceCandidate(string payload)
        {
            var (peerId, candidateJson) = SplitOnce(payload);
            signaling.SendIceCandidate(peerId, candidateJson);
        }

        public void OnJsLocalDescription(string payload)
        {
            var parts = payload.Split('|', 3);
            var peerId = parts[0];
            var type = parts[1];
            var sdp = parts[2];

            if (type == "offer") signaling.SendOffer(peerId, sdp);
            else signaling.SendAnswer(peerId, sdp);
        }

        public void OnJsChannelOpen(string peerId) => PeerConnected?.Invoke(peerId);

        public void OnJsDataReceived(string payload)
        {
            var (peerId, base64) = SplitOnce(payload);
            DataReceived?.Invoke(peerId, Convert.FromBase64String(base64));
        }

        static (string, string) SplitOnce(string payload)
        {
            var index = payload.IndexOf('|');
            return (payload[..index], payload[(index + 1)..]);
        }
    }
}
#endif
