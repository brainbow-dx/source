using System.Collections;
using System.Collections.Generic;

using UnityEngine;
using UnityEngine.SceneManagement;

using UnityEditor;
using UnityEditor.Build.Reporting;

using Unity.Runtime;
using UnityEditor.SceneManagement;

namespace Unity.Editor.Aby.Actions
{
    /// <summary>
    /// Encapsulates static build actions for the Aby Unity runtime.
    /// </summary>
    public static class Build
    {
        /// <summary>
        /// Opens the Sandbox scene. The ecma engine initializes automatically on entering Play
        /// mode, see <c>Escher.Unity.Editor.EditorLifecycle</c>.
        /// </summary>
        public static void Sandbox()
        {
            var sceneName = "Sandbox";
            EditorSceneManager.OpenScene("Assets/Scenes/" + sceneName + ".unity");
            Debug.LogFormat("Loaded Scene `{0}`", sceneName);
        }

        /// <summary>
        /// Batchmode entry point (<c>-executeMethod Unity.Editor.Aby.Actions.Build.WebGL</c>) for
        /// <c>scripts/webgl-multiplayer-test.sh</c> — builds to <c>.output/web</c> with
        /// compression disabled so the result can be served by any plain static file server (no
        /// gzip/brotli Content-Encoding headers needed). Not a release build config.
        /// </summary>
        public static void WebGL()
        {
            var previousCompression = PlayerSettings.WebGL.compressionFormat;
            PlayerSettings.WebGL.compressionFormat = WebGLCompressionFormat.Disabled;

            try
            {
                var outputPath = System.IO.Path.Combine(Application.dataPath, "..", ".output", "web");
                var scenes = System.Array.ConvertAll(EditorBuildSettings.scenes, scene => scene.path);

                var report = BuildPipeline.BuildPlayer(new BuildPlayerOptions
                {
                    scenes = scenes,
                    locationPathName = outputPath,
                    target = BuildTarget.WebGL,
                    options = BuildOptions.None,
                });

                if (report.summary.result != BuildResult.Succeeded)
                {
                    throw new System.Exception(
                        $"WebGL build did not succeed: {report.summary.result} ({report.summary.totalErrors} errors)");
                }

                Debug.LogFormat("WebGL build succeeded: {0}", outputPath);
            }
            finally
            {
                PlayerSettings.WebGL.compressionFormat = previousCompression;
            }
        }
    }
}
