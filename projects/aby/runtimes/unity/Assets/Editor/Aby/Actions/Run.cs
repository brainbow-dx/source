using System.IO;

using UnityEngine;

using Escher.Unity;
using Unity.Runtime;

namespace Unity.Editor.Aby.Actions
{
    /// <summary>
    /// Editor actions for running the embedded ecma engine outside Play mode.
    /// </summary>
    public static class Run
    {
        /// <summary>
        /// Initializes the ecma engine if needed, then runs the Counter example module.
        /// </summary>
        public static void Server()
        {
            if (!EscherRuntime.IsInitialized)
            {
                EscherRuntime.InitializeInEditor();
            }

            string modulePath = Path.Combine(Application.dataPath, "..", "Examples", "Counter", "main.js");
            CStartResult result = EscherRuntime.ExecuteModule(modulePath);

            if (result != CStartResult.Ok)
            {
                Debug.LogError($"[Escher] Module execution failed: {result}");
            }
            else
            {
                Debug.Log("[Escher] Module executed successfully.");
            }
        }
    }
}
