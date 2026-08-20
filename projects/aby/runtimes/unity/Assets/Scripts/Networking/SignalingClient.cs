using System;
using System.Text;
using NativeWebSocket;
using UnityEngine;

namespace Aby.Networking
{
    /// <summary>
    /// The WebSocket connection to <c>atlas-relay</c>, shared by both <see
    /// cref="NativeWebRtcTransport"/> and <see cref="WebGLWebRtcTransport"/> — the signaling
    /// protocol itself has nothing platform-specific about it, only the actual WebRTC
    /// peer-connection APIs differ per platform. Wraps <c>NativeWebSocket</c> rather than raw
    /// <c>System.Net.WebSockets</c> specifically because it already has a working WebGL backend
    /// (a bundled <c>.jslib</c>), so this one class works unmodified on every platform this
    /// project targets.
    /// </summary>
    public class SignalingClient
    {
        readonly WebSocket socket;

        public string LocalPeerId { get; private set; }

        public event Action<string[]> Joined;
        public event Action<string> PeerJoined;
        public event Action<string> PeerLeft;
        public event Action<string, string> OfferReceived; // (fromPeerId, sdp)
        public event Action<string, string> AnswerReceived; // (fromPeerId, sdp)
        public event Action<string, string> IceCandidateReceived; // (fromPeerId, candidate)

        string pendingRoom;

        public SignalingClient(string relayUrl)
        {
            socket = new WebSocket(relayUrl);
            socket.OnMessage += OnMessage;
            socket.OnError += error => Debug.LogError($"[Signaling] error: {error}");
            socket.OnOpen += () =>
            {
                if (pendingRoom != null)
                {
                    Send(SignalingProtocol.Join(pendingRoom));
                }
            };
        }

        public void Connect(string room)
        {
            pendingRoom = room;
            _ = socket.Connect();
        }

        public void Disconnect() => _ = socket.Close();

        /// <summary>
        /// Pumps queued messages on platforms where <c>NativeWebSocket</c> can't dispatch them
        /// on its own background thread (everywhere except WebGL, where the browser's own event
        /// loop delivers them instead). Call once per frame, e.g. from a <c>MonoBehaviour</c>'s
        /// <c>Update</c>. Harmless, and a no-op, on WebGL.
        /// </summary>
        public void DispatchMessageQueue()
        {
#if !UNITY_WEBGL || UNITY_EDITOR
            socket.DispatchMessageQueue();
#endif
        }

        public void SendOffer(string to, string sdp) => Send(SignalingProtocol.Offer(to, sdp));
        public void SendAnswer(string to, string sdp) => Send(SignalingProtocol.Answer(to, sdp));
        public void SendIceCandidate(string to, string candidate) => Send(SignalingProtocol.IceCandidate(to, candidate));

        void Send(string json) => _ = socket.SendText(json);

        void OnMessage(byte[] bytes)
        {
            var message = ServerMessage.Parse(Encoding.UTF8.GetString(bytes));
            switch (message.Type)
            {
                case "joined":
                    LocalPeerId = message.PeerId;
                    Joined?.Invoke(message.Peers ?? Array.Empty<string>());
                    break;
                case "peer-joined":
                    PeerJoined?.Invoke(message.PeerId);
                    break;
                case "peer-left":
                    PeerLeft?.Invoke(message.PeerId);
                    break;
                case "offer":
                    OfferReceived?.Invoke(message.From, message.Sdp);
                    break;
                case "answer":
                    AnswerReceived?.Invoke(message.From, message.Sdp);
                    break;
                case "ice-candidate":
                    IceCandidateReceived?.Invoke(message.From, message.Candidate);
                    break;
                case "error":
                    Debug.LogError($"[Signaling] relay error: {message.Message}");
                    break;
                default:
                    Debug.LogWarning($"[Signaling] unrecognized message type: {message.Type}");
                    break;
            }
        }
    }
}
