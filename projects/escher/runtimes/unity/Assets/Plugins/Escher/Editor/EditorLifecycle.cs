#if UNITY_EDITOR
using UnityEditor;

namespace Escher.Unity.Editor
{
    /// <summary>
    /// Hooks Editor lifecycle events so the native runtime is torn down before a domain reload
    /// invalidates the native pointer it holds. A stale pointer after reload crashes.
    /// </summary>
    [InitializeOnLoad]
    static class EditorLifecycle
    {
        static EditorLifecycle()
        {
            AssemblyReloadEvents.beforeAssemblyReload += OnBeforeAssemblyReload;
            EditorApplication.quitting += OnQuitting;
            EditorApplication.playModeStateChanged += OnPlayModeStateChanged;
        }

        static void OnBeforeAssemblyReload()
        {
            EscherRuntime.Shutdown();
        }

        static void OnQuitting()
        {
            EscherRuntime.Shutdown();
        }

        static void OnPlayModeStateChanged(PlayModeStateChange state)
        {
            switch (state)
            {
                case PlayModeStateChange.EnteredPlayMode:
                    EscherRuntime.InitializeInEditor();
                    break;

                case PlayModeStateChange.ExitingPlayMode:
                    EscherRuntime.Shutdown();
                    break;
            }
        }
    }
}
#endif
