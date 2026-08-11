using System;
using System.Collections;
using System.Collections.Generic;
using System.Runtime.InteropServices;

using UnityEngine;
using UnityEditor;

namespace Unity.Runtime
{
    /// <summary>
    /// TODO
    /// </summary>
    public class AbyRuntime
    {
        /// <summary>
        /// TODO
        /// </summary>
        public static bool ShouldShowBillboard
        {
#if UNITY_EDITOR
            get => EditorPrefs.GetBool("ShowBillboard", true);
            set => EditorPrefs.SetBool("ShowBillboard", value);
#else
            get => false;
            set => Debug.LogWarning("Not implemented!");
#endif
        }

        /// <summary>
        /// TODO
        /// </summary>
        public static bool HasShownBillboard
        {
#if UNITY_EDITOR
            get => SessionState.GetBool("HasShownBillboard", false);
            set => SessionState.SetBool("HasShownBillboard", value);
#else
            get => false;
            set => Debug.LogWarning("Not implemented!");
#endif
        }

        /// <summary>
        /// TODO
        /// </summary>
        public const string THETA_SETTINGS_LABEL = "Aby";

        //--
        /// <summary>
        /// TODO
        /// </summary>
        // [InitializeOnLoadMethod()]
        // private static void Billboard()
        // {
        //     if (ShouldShowBillboard && !HasShownBillboard)
        //     {
        //         Debug.LogFormat("Aby SDK Assembly Name: {0}", Aby.SDK.Config.AssemblyName);
        //         HasShownBillboard = true;
        //     }
        // }

        /// <summary>
        /// TODO
        /// </summary>
        /// <returns>An instance of SettingsProvider (for Unity Editor)</returns>
        // [SettingsProvider]
        // public static SettingsProvider CreateSettingsProvider()
        // {
        //     var provider = new SettingsProvider("Preferences/Aby", SettingsScope.User)
        //     {
        //         label = THETA_SETTINGS_LABEL,
        //         keywords = new HashSet<string>(new[] { "Aby", "Aby" }),
        //         guiHandler = ctx =>
        //         {
        //             EditorGUILayout.BeginHorizontal();
        //             EditorGUILayout.LabelField("Show Billboard", GUILayout.Width(200));

        //             var showBillbaordPref = EditorGUILayout.Toggle(ShouldShowBillboard);

        //             EditorGUILayout.EndHorizontal();

        //             if (GUI.changed)
        //             {
        //                 ShouldShowBillboard = showBillbaordPref;
        //             }
        //         },
        //     };

        //     return provider;
        // }
    }
}
