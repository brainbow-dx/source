using System;
using Newtonsoft.Json;

namespace Aby.Networking
{
    /// <summary>
    /// Mirrors <c>atlas/packages/relay/src/protocol.rs</c> exactly. That crate is the source of
    /// truth for this wire format, not this file.
    /// </summary>
    public static class SignalingProtocol
    {
        public static string Join(string room) =>
            JsonConvert.SerializeObject(new { type = "join", room });

        public static string Offer(string to, string sdp) =>
            JsonConvert.SerializeObject(new { type = "offer", to, sdp });

        public static string Answer(string to, string sdp) =>
            JsonConvert.SerializeObject(new { type = "answer", to, sdp });

        public static string IceCandidate(string to, string candidate) =>
            JsonConvert.SerializeObject(new { type = "ice-candidate", to, candidate });
    }

    /// <summary>
    /// A parsed server-to-client message. Only the fields relevant to <see cref="Type"/> are
    /// populated, matching the Rust side's tagged enum.
    /// </summary>
    [Serializable]
    public class ServerMessage
    {
        [JsonProperty("type")]
        public string Type;

        [JsonProperty("peer_id")]
        public string PeerId;

        [JsonProperty("peers")]
        public string[] Peers;

        [JsonProperty("from")]
        public string From;

        [JsonProperty("sdp")]
        public string Sdp;

        [JsonProperty("candidate")]
        public string Candidate;

        [JsonProperty("message")]
        public string Message;

        public static ServerMessage Parse(string json) => JsonConvert.DeserializeObject<ServerMessage>(json);
    }
}
