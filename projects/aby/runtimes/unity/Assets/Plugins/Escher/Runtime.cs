using System;
using System.Runtime.InteropServices;
using Unity.Runtime;
using UnityEngine;

namespace Escher.Unity
{
    /// <summary>
    /// Owns a single global ethos-ecma runtime instance for the whole process.
    /// </summary>
    public static unsafe class EscherRuntime
    {
#if !UNITY_WEBGL
        static CEcmaRuntime* runtime;

        // Kept alive as a static field so the GC never collects it while native code holds the
        // function pointer obtained from it below.
        static readonly EcmaRuntime.c_verify_log_callback__cb_delegate logCallback = OnLog;

        static void OnLog(byte* message)
        {
            string text = message is null ? "<null>" : (Marshal.PtrToStringUTF8((IntPtr)message) ?? "<invalid utf8>");
            Debug.Log($"[Escher] {text}");
        }

        public static bool IsInitialized => runtime != null;

        public static void Initialize(string dbDir, string logDir)
        {
            if (IsInitialized)
            {
                Debug.LogWarning("[Escher] Runtime already initialized.");
                return;
            }

            byte[] dbDirBytes = System.Text.Encoding.UTF8.GetBytes(dbDir + "\0");
            byte[] logDirBytes = System.Text.Encoding.UTF8.GetBytes(logDir + "\0");

            fixed (byte* dbDirPtr = dbDirBytes)
            fixed (byte* logDirPtr = logDirBytes)
            {
                var config = new CEcmaRuntimeConfig
                {
                    inspector_wait = false,
                    inspector_port = 9222,
                    db_dir = dbDirPtr,
                    log_dir = logDirPtr,
                    log_level = CEcmaRuntimeLogLevel.Info,
                    log_callback_fn = (void*)Marshal.GetFunctionPointerForDelegate(logCallback),
                };

                runtime = EcmaRuntime.c_construct_runtime(config);
            }

            if (runtime == null)
            {
                Debug.LogError("[Escher] Failed to construct runtime.");
            }
        }

        public static CStartResult ExecuteModule(string modulePath)
        {
            if (!IsInitialized)
            {
                Debug.LogError("[Escher] Runtime not initialized; call Initialize first.");
                return CStartResult.BindingErr;
            }

            byte[] moduleBytes = System.Text.Encoding.UTF8.GetBytes(modulePath + "\0");

            fixed (byte* modulePtr = moduleBytes)
            {
                var options = new CExecuteModuleOptions { main_module_specifier = modulePtr };
                return EcmaRuntime.c_exec_module(runtime, options);
            }
        }

        /// <summary>
        /// Frees the native runtime. Call before Unity invalidates any native pointers this holds,
        /// on domain reload or process exit. See <c>Editor/EditorLifecycle.cs</c> for the hook.
        /// </summary>
        public static void Shutdown()
        {
            if (!IsInitialized) return;

            EcmaRuntime.c_free_runtime(runtime);
            runtime = null;
        }
#else
        // The native plugin (`libecma`) is a desktop dylib/so — there is no WebGL/wasm build of
        // it yet, so every entry point here is a safe no-op instead of a P/Invoke into a library
        // that can't exist for this platform. Revisit once ethos-ecma targets wasm32.
        public static bool IsInitialized => false;

        public static void Initialize(string dbDir, string logDir)
        {
            Debug.LogWarning("[Escher] Runtime not available on WebGL yet.");
        }

        public static CStartResult ExecuteModule(string modulePath)
        {
            Debug.LogWarning("[Escher] Runtime not available on WebGL yet.");
            return CStartResult.BindingErr;
        }

        public static void Shutdown() { }
#endif

#if UNITY_EDITOR
        /// <summary>
        /// Initializes with the standard Editor-side data/log directories, under the project's
        /// Library folder. Shared by every Editor call site so they all use the same paths.
        /// </summary>
        public static void InitializeInEditor()
        {
            string projectRoot = System.IO.Path.GetDirectoryName(UnityEngine.Application.dataPath);
            Initialize(
                System.IO.Path.Combine(projectRoot, "Library", "EscherData"),
                System.IO.Path.Combine(projectRoot, "Library", "EscherLogs")
            );
        }
#else
        // Player builds have no domain-reload concern and no Editor lifecycle events. Bootstrap
        // once before the first scene loads, and rely on process exit for teardown.
        [RuntimeInitializeOnLoadMethod(RuntimeInitializeLoadType.BeforeSceneLoad)]
        static void InitializeForPlayer()
        {
            Initialize(
                System.IO.Path.Combine(Application.persistentDataPath, "EscherData"),
                System.IO.Path.Combine(Application.persistentDataPath, "EscherLogs")
            );
        }
#endif
    }
}
