using System;
using System.IO;
using System.Linq;

namespace ToolsBoXApp;

public static class ContentPaths
{
    public const string FolderName = "contents";

    public static string? Resolve()
    {
        var env = Environment.GetEnvironmentVariable("TOOLSBOX_CONTENT_DIR");
        if (!string.IsNullOrWhiteSpace(env) && Directory.Exists(env))
            return Path.GetFullPath(env);

        var localOverride = Path.Combine(
            Environment.GetFolderPath(Environment.SpecialFolder.LocalApplicationData),
            "ToolsBoXApp", FolderName);
        if (Directory.Exists(localOverride))
            return localOverride;

        var beside = Path.Combine(AppContext.BaseDirectory, FolderName);
        if (Directory.Exists(beside))
            return beside;

        var dir = new DirectoryInfo(AppContext.BaseDirectory);
        for (var i = 0; i < 8 && dir is not null; i++, dir = dir.Parent)
        {
            var candidate = Path.Combine(dir.FullName, FolderName);
            if (Directory.Exists(candidate) && File.Exists(Path.Combine(candidate, "index.html")))
                return candidate;
        }

        return null;
    }
}