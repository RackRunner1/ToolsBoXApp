using System;
using System.Collections.Generic;
using System.Collections.ObjectModel;
using System.ComponentModel;
using System.IO;
using System.Linq;

namespace ToolsBoXApp;

/// <summary>Nœud de l'arborescence contents/ (dossier ou fichier), chargé paresseusement.</summary>
public sealed class ContentNode : INotifyPropertyChanged
{
    private readonly string _root;
    private bool _childrenLoaded;
    private bool _isExpanded;

    private ContentNode(string fullPath, string root)
    {
        FullPath = fullPath;
        _root = root;
        Name = Path.GetFileName(fullPath.TrimEnd(Path.DirectorySeparatorChar, Path.AltDirectorySeparatorChar));
        IsDir = Directory.Exists(fullPath);
        IsHtml = !IsDir && Name.EndsWith(".html", StringComparison.OrdinalIgnoreCase);

        var rel = Path.GetRelativePath(root, fullPath).Replace('\\', '/');
        RelativePath = rel;
    }

    public string Name { get; }
    public string FullPath { get; }
    public string RelativePath { get; }
    public bool IsDir { get; }
    public bool IsHtml { get; }

    public string Display => (IsDir ? "\U0001F4C1 " : IsHtml ? "\U0001F310 " : "\U0001F4C4 ") + Name;
    public string Foreground => IsHtml ? "#7dd3fc" : IsDir ? "#e2e8f0" : "#64748b";

    public ObservableCollection<ContentNode> Children { get; } = new();

    public bool IsExpanded
    {
        get => _isExpanded;
        set
        {
            if (_isExpanded == value) return;
            _isExpanded = value;
            OnPropertyChanged(nameof(IsExpanded));
            if (_isExpanded) EnsureChildren();
        }
    }

    /// <summary>Charge les enfants d'un dossier une seule fois.</summary>
    public void EnsureChildren()
    {
        if (_childrenLoaded || !IsDir) return;
        _childrenLoaded = true;
        foreach (var child in Enumerate(FullPath, _root))
            Children.Add(child);
    }

    /// <summary>Scanne le contenu racine (dossiers d'abord, puis fichiers, triés).</summary>
    public static IEnumerable<ContentNode> Scan(string root) => Enumerate(root, root);

    private static IEnumerable<ContentNode> Enumerate(string dir, string root)
    {
        IEnumerable<string> dirs;
        IEnumerable<string> files;
        try
        {
            dirs = Directory.EnumerateDirectories(dir);
            files = Directory.EnumerateFiles(dir);
        }
        catch (Exception) // dossier inaccessible : on ignore silencieusement
        {
            yield break;
        }

        foreach (var d in dirs.OrderBy(x => x, StringComparer.OrdinalIgnoreCase))
            yield return new ContentNode(d, root);
        foreach (var f in files.OrderBy(x => x, StringComparer.OrdinalIgnoreCase))
            yield return new ContentNode(f, root);
    }

    public event PropertyChangedEventHandler? PropertyChanged;

    private void OnPropertyChanged(string name) =>
        PropertyChanged?.Invoke(this, new PropertyChangedEventArgs(name));
}
