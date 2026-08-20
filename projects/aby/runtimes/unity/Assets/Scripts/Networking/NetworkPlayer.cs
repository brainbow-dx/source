using System.Text;
using UnityEngine;

namespace Aby.Networking
{
    /// <summary>
    /// The minimal proof that the whole pipe works: join a room via <c>atlas-relay</c>, connect
    /// to whichever peers are there over a real WebRTC data channel (native or WebGL, whichever
    /// this build is), and exchange one message. Not a real networked-player implementation,
    /// actual game-state replication (position, animation, etc.) is separate follow-up work on
    /// top of the connection this proves out. See the plan's "explicitly out of scope" section.
    /// </summary>
    public class NetworkPlayer : MonoBehaviour
    {
        [SerializeField] string relayUrl = "ws://localhost:9200/ws";
        [SerializeField] string room = "aby-dev";

        IPeerTransport transport;

        void Start()
        {
            transport = PeerTransportFactory.AddTo(gameObject);

            transport.PeerJoined += peerId => Debug.Log($"[NetworkPlayer] peer joined: {peerId}");
            transport.PeerLeft += peerId => Debug.Log($"[NetworkPlayer] peer left: {peerId}");
            transport.PeerConnected += peerId =>
            {
                Debug.Log($"[NetworkPlayer] data channel open to {peerId}, sending hello");
                transport.SendData(peerId, Encoding.UTF8.GetBytes($"hello from {transport.LocalPeerId}"));
            };
            transport.DataReceived += (peerId, data) =>
            {
                Debug.Log($"[NetworkPlayer] received from {peerId}: {Encoding.UTF8.GetString(data)}");
            };

            transport.Connect(relayUrl, room);
        }

        void OnDestroy() => transport?.Disconnect();
    }
}
