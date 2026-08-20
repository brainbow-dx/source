using System;

namespace Aby.Networking
{
    /// <summary>
    /// One connection to a room of peers, talking WebRTC data channels underneath. Game code
    /// depends only on this interface, never on <see cref="NativeWebRtcTransport"/> or
    /// <see cref="WebGLWebRtcTransport"/> directly — which backend gets used is a build-platform
    /// decision, not a gameplay one. See <c>atlas/packages/relay</c> for the signaling server
    /// both backends talk to.
    /// </summary>
    public interface IPeerTransport
    {
        /// <summary>This peer's own ID, assigned by the relay once <see cref="Connect"/>'s join
        /// completes. Null until then.</summary>
        string LocalPeerId { get; }

        /// <summary>Connects to the relay at <paramref name="relayUrl"/> (e.g.
        /// <c>ws://localhost:9200/ws</c>) and joins <paramref name="room"/>.</summary>
        void Connect(string relayUrl, string room);

        void Disconnect();

        /// <summary>Sends <paramref name="data"/> to <paramref name="peerId"/> over that peer's
        /// data channel. No-ops if no data channel is open to that peer yet.</summary>
        void SendData(string peerId, byte[] data);

        /// <summary>A peer joined the room. A connection attempt to them starts automatically;
        /// this does not mean a data channel is open yet, see <see cref="PeerConnected"/>.
        /// </summary>
        event Action<string> PeerJoined;

        /// <summary>A peer left the room, or their connection was lost.</summary>
        event Action<string> PeerLeft;

        /// <summary>The data channel to a peer actually opened. Safe to <see cref="SendData"/>
        /// to them from this point on.</summary>
        event Action<string> PeerConnected;

        /// <summary>A message arrived from a peer over their data channel.</summary>
        event Action<string, byte[]> DataReceived;
    }
}
