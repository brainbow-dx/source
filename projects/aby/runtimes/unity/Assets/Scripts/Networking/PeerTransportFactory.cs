using UnityEngine;

namespace Aby.Networking
{
    /// <summary>
    /// Adds whichever <see cref="IPeerTransport"/> implementation is real on the current
    /// platform to <paramref name="host"/> and returns it. The only place in game code that
    /// needs to know both backends exist, everything downstream of this just holds an
    /// <see cref="IPeerTransport"/>.
    /// </summary>
    public static class PeerTransportFactory
    {
        public static IPeerTransport AddTo(GameObject host)
        {
#if UNITY_WEBGL && !UNITY_EDITOR
            return host.AddComponent<WebGLWebRtcTransport>();
#else
            return host.AddComponent<NativeWebRtcTransport>();
#endif
        }
    }
}
