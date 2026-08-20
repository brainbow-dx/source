using System.ComponentModel;

using UnityEngine;
using UnityEngine.UIElements;
using UnityEngine.SceneManagement;

using UnityEditor;
using UnityEditor.UIElements;
using UnityEditor.SceneManagement;

using Escher.Unity;
using Unity.Runtime;

namespace Unity.Editor.Aby
{
    /// <summary>
    /// TODO
    /// </summary>\
    [InitializeOnLoad]
    public class AbyControllerEditorWindow : EditorWindow
    {
        /// <summary>
        /// TODO
        /// </summary>
        [System.Serializable]
        public class State
        {
            /// <summary>
            /// TODO
            /// </summary>
            [SerializeField]
            public string Status;
        }

        /// <summary>
        /// TODO
        /// </summary>
        const int DEFAULT_SERVICE_PORT = 9000;

        /// <summary>
        /// The expected Exit Code used to ask the dev script to reload
        /// the window, instead of just shutting down.
        /// </summary>
        const int RELOAD_CLIENT_EXIT_CODE = 100;

        /// <summary>
        /// TODO
        /// </summary>
        [SerializeField]
        private State m_State = new State();

        /// <summary>
        /// TODO
        /// </summary>
        [SerializeField]
        private VisualTreeAsset m_VisualTreeAsset = default;

        /// <summary>
        /// TODO
        /// </summary>
        static AbyControllerEditorWindow()
        {
            // Mount a heirarchy gui event handler.
            EditorApplication.hierarchyWindowItemOnGUI += OnHierarchyWindowItemGUI;
        }

        /// <summary>
        /// TODO
        /// </summary>
        [MenuItem("Aby/Aby Controller")]
        public static void ShowWindow()
        {
            var abyControllerWindow = GetWindow<AbyControllerEditorWindow>();
            abyControllerWindow.titleContent = new GUIContent("Aby Controller");
        }

        //--
        /// <summary>
        /// TODO
        /// </summary>
        private void OnDestroy()
        {
            // TODO
        }

        //--
        /// <summary>
        /// Mounts the root `VisualElement` and inits the start/reload buttons.
        /// </summary>
        public void CreateGUI()
        {
            if (m_VisualTreeAsset != null)
            {
                rootVisualElement.Add(m_VisualTreeAsset.Instantiate());
                DrawRuntimeControlToolbar();
            }
            else
            {
                Debug.LogError("VisualTreeAsset is not assigned.");
            }
        }
        /// <summary>
        /// TODO
        /// </summary>
        private void DrawRuntimeControlToolbar()
        {
            var toggleButton = rootVisualElement.Q<Button>("ToggleButton");
            if (toggleButton == null)
            {
                Debug.LogError("ToggleButton element not found!");
            }
            else
            {
                toggleButton.clicked += OnToggleButtonClicked;
            }

            var reloadButton = rootVisualElement.Q<Button>("ReloadButton");
            if (reloadButton == null)
            {
                Debug.LogError("ReloadButton element not found!");
            }
            else
            {
                reloadButton.clicked += OnReloadButtonClicked;
            }

            RefreshRuntimeStatus();
        }

        private void RefreshRuntimeStatus()
        {
            var stateLabel = rootVisualElement.Q<Label>("RuntimeState");
            if (stateLabel != null)
            {
                stateLabel.text = $"Runtime State: {(EscherRuntime.IsInitialized ? "Initialized" : "Not Initialized")}";
            }

            var toggleButton = rootVisualElement.Q<Button>("ToggleButton");
            if (toggleButton != null)
            {
                toggleButton.text = EscherRuntime.IsInitialized ? "Stop" : "Start";
            }
        }

        /// <summary>
        /// TODO
        /// </summary>
        private static void OnHierarchyWindowItemGUI(int instanceID, Rect selectionRect)
        {
            // Debug.LogFormat("Found GUI item: {0}", instanceID);
        }

        private void OnToggleButtonClicked()
        {
            if (!EscherRuntime.IsInitialized)
            {
                EscherRuntime.InitializeInEditor();
            }
            else
            {
                EscherRuntime.Shutdown();
            }

            RefreshRuntimeStatus();
        }

        /// <summary>
        /// TODO
        /// </summary>
        private void OnReloadButtonClicked()
        {
            Debug.Log("Attempting to reload plugin.");

            // We need to exit play mode first so we can (optionally) save
            // and safely run shutdown operations.
            EditorApplication.ExitPlaymode();

            // `ExitPlaymore` doesn't complete until "later", so we defer
            // the rest of the operation until then.
            EditorApplication.delayCall += DelayedOnReloadButtonClicked;
        }

        /// <summary>
        /// TODO
        /// </summary>
        private void DelayedOnReloadButtonClicked()
        {
            EditorApplication.delayCall -= DelayedOnReloadButtonClicked;

            if (EditorSceneManager.SaveCurrentModifiedScenesIfUserWantsTo())
            {
                if (ConfirmEditorRestart())
                {
                    EditorApplication.Exit(RELOAD_CLIENT_EXIT_CODE); // Request reload.
                }
            }
        }

        //--
        /// <summary>
        /// TODO
        /// </summary>
        private bool ConfirmEditorRestart()
        {
            return EditorUtility.DisplayDialog(
                "Restart Editor?",
                $"You'll need to restart the editor before changes take effect.",
                "Yes pls! (Recommended)",
                "No thx."
            );
        }
    }
}
