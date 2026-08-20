#if !UNITY_WEBGL || UNITY_EDITOR
using System;
using System.Collections;
using System.Collections.Generic;
using Newtonsoft.Json;
using Unity.WebRTC;
using UnityEngine;

namespace Aby.Networking
{
    /// <summary>
    /// <see cref="IPeerTransport"/> backed by <c>com.unity.webrtc</c> (native libwebrtc). Used
    /// for the Editor, PC/Mac standalone, and mobile builds — everywhere except WebGL, which
    /// can't load native plugins and uses <see cref="WebGLWebRtcTransport"/> instead. Excluded
    /// from WebGL builds entirely by the <c>#if</c> above (kept for the Editor even when the
    /// active build target is WebGL, so switching platforms in the Editor doesn't also require
    /// recompiling scripts).
    /// </summary>
    public class NativeWebRtcTransport : MonoBehaviour, IPeerTransport
    {
        static readonly RTCConfiguration Configuration = new RTCConfiguration
        {
            iceServers = new[] { new RTCIceServer { urls = new[] { "stun:stun.l.google.com:19302" } } },
        };

        SignalingClient signaling;
        readonly Dictionary<string, RTCPeerConnection> connections = new();
        readonly Dictionary<string, RTCDataChannel> channels = new();

        public string LocalPeerId => signaling?.LocalPeerId;

        public event Action<string> PeerJoined;
        public event Action<string> PeerLeft;
        public event Action<string> PeerConnected;
        public event Action<string, byte[]> DataReceived;

        void Awake()
        {
            StartCoroutine(WebRTC.Update());
        }

        public void Connect(string relayUrl, string room)
        {
            signaling = new SignalingClient(relayUrl);
            signaling.Joined += existingPeers =>
            {
                foreach (var peerId in existingPeers)
                {
                    StartCoroutine(InitiateConnection(peerId));
                }
            };
            signaling.PeerJoined += peerId => PeerJoined?.Invoke(peerId);
            signaling.PeerLeft += peerId =>
            {
                ClosePeer(peerId);
                PeerLeft?.Invoke(peerId);
            };
            signaling.OfferReceived += (from, sdp) => StartCoroutine(HandleOffer(from, sdp));
            signaling.AnswerReceived += (from, sdp) => StartCoroutine(HandleAnswer(from, sdp));
            signaling.IceCandidateReceived += (from, candidateJson) => HandleRemoteIceCandidate(from, candidateJson);

            signaling.Connect(room);
        }

        public void Disconnect()
        {
            foreach (var peerId in new List<string>(connections.Keys))
            {
                ClosePeer(peerId);
            }
            signaling?.Disconnect();
        }

        public void SendData(string peerId, byte[] data)
        {
            if (channels.TryGetValue(peerId, out var channel) && channel.ReadyState == RTCDataChannelState.Open)
            {
                channel.Send(data);
            }
        }

        void Update() => signaling?.DispatchMessageQueue();

        /// <summary>The side that was already in the room when a new peer joins is the one that
        /// initiates, so exactly one offer gets created per pair, not two racing each other.
        /// </summary>
        IEnumerator InitiateConnection(string peerId)
        {
            var pc = CreatePeerConnection(peerId);
            var channel = pc.CreateDataChannel("data");
            WireDataChannel(peerId, channel);

            var offerOptions = default(RTCOfferAnswerOptions);
            var offerOp = pc.CreateOffer(ref offerOptions);
            yield return offerOp;

            var offerDesc = offerOp.Desc;
            var setLocalOp = pc.SetLocalDescription(ref offerDesc);
            yield return setLocalOp;

            signaling.SendOffer(peerId, offerDesc.sdp);
        }

        IEnumerator HandleOffer(string fromPeerId, string sdp)
        {
            var pc = CreatePeerConnection(fromPeerId);

            var remoteDesc = new RTCSessionDescription { type = RTCSdpType.Offer, sdp = sdp };
            yield return pc.SetRemoteDescription(ref remoteDesc);

            var answerOptions = default(RTCOfferAnswerOptions);
            var answerOp = pc.CreateAnswer(ref answerOptions);
            yield return answerOp;

            var answerDesc = answerOp.Desc;
            yield return pc.SetLocalDescription(ref answerDesc);

            signaling.SendAnswer(fromPeerId, answerDesc.sdp);
        }

        IEnumerator HandleAnswer(string fromPeerId, string sdp)
        {
            if (!connections.TryGetValue(fromPeerId, out var pc)) yield break;

            var remoteDesc = new RTCSessionDescription { type = RTCSdpType.Answer, sdp = sdp };
            yield return pc.SetRemoteDescription(ref remoteDesc);
        }

        void HandleRemoteIceCandidate(string fromPeerId, string candidateJson)
        {
            if (!connections.TryGetValue(fromPeerId, out var pc)) return;

            var bundle = JsonConvert.DeserializeObject<IceCandidateBundle>(candidateJson);
            var init = new RTCIceCandidateInit
            {
                candidate = bundle.candidate,
                sdpMid = bundle.sdpMid,
                sdpMLineIndex = bundle.sdpMLineIndex,
            };
            pc.AddIceCandidate(new RTCIceCandidate(init));
        }

        RTCPeerConnection CreatePeerConnection(string peerId)
        {
            var config = Configuration;
            var pc = new RTCPeerConnection(ref config);

            pc.OnIceCandidate = candidate =>
            {
                var bundle = new IceCandidateBundle
                {
                    candidate = candidate.Candidate,
                    sdpMid = candidate.SdpMid,
                    sdpMLineIndex = candidate.SdpMLineIndex ?? 0,
                };
                signaling.SendIceCandidate(peerId, JsonConvert.SerializeObject(bundle));
            };

            // Only fires on the answering side — the offering side already has its own channel
            // reference from CreateDataChannel in InitiateConnection.
            pc.OnDataChannel = channel => WireDataChannel(peerId, channel);

            connections[peerId] = pc;
            return pc;
        }

        void WireDataChannel(string peerId, RTCDataChannel channel)
        {
            channels[peerId] = channel;
            channel.OnOpen += () => PeerConnected?.Invoke(peerId);
            channel.OnMessage += bytes => DataReceived?.Invoke(peerId, bytes);
        }

        void ClosePeer(string peerId)
        {
            if (channels.Remove(peerId, out var channel)) channel.Close();
            if (connections.Remove(peerId, out var pc)) pc.Close();
        }

        [Serializable]
        class IceCandidateBundle
        {
            public string candidate;
            public string sdpMid;
            public int sdpMLineIndex;
        }
    }
}
#endif
