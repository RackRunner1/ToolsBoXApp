using System;
using System.Diagnostics;
using System.IO;
using System.Text.Json;
using System.Threading.Tasks;
using System.Windows;
using System.Windows.Controls;
using System.Windows.Input;
using Microsoft.Web.WebView2.Core;
using Microsoft.Web.WebView2.Wpf;

namespace ToolsBoXApp;

public partial class MainWindow : Window
{
    private const string VirtualHost = "toolbox.local";
    private string? _contentsDir;

    public MainWindow()
    {
        InitializeComponent();
        Loaded += async (_, _) => await OnLoadedAsync();
        PreviewKeyDown += MainWindow_PreviewKeyDown;
    }

    private async Task OnLoadedAsync()
    {
        _contentsDir = ContentPaths.Resolve();

        if (_contentsDir is null)
        {
            Placeholder.Text =
                "Dossier contents/ introuvable.\n\n" +
                "Il est normalement synchronisé chaque nuit depuis ToolsBoXMachine " +
                "et distribué dans l'application.\n" +
                "(Variable d'environnement TOOLSBOX_CONTENT_DIR pour pointer un dossier personnalisé.)";
            TxtStatus.Text = "contents/ introuvable";
            return;
        }

        try
        {
            var userDataFolder = Path.Combine(
                Environment.GetFolderPath(Environment.SpecialFolder.LocalApplicationData),
                "ToolsBoXApp", "WebView2");
            var env = await CoreWebView2Environment.CreateAsync(null, userDataFolder);
            await Browser.EnsureCoreWebView2Async(env);
        }
        catch (Exception ex)
        {
            Placeholder.Text =
                "Le moteur WebView2 (Microsoft Edge) n'est pas disponible sur ce PC.\n\n" +
                "Installez-le : https://developer.microsoft.com/microsoft-edge/webview2/\n\n" + ex.Message;
            TxtStatus.Text = "WebView2 manquant";
            return;
        }

        var core = Browser.CoreWebView2!;

        core.SetVirtualHostNameToFolderMapping(
            VirtualHost, _contentsDir, CoreWebView2HostResourceAccessKind.Allow);

        core.NewWindowRequested += (_, e) =>
        {
            try { Process.Start(new ProcessStartInfo(e.Uri) { UseShellExecute = true }); }
            catch { }
            e.Handled = true;
        };

        Browser.NavigationCompleted += (_, e) =>
        {
            if (e.IsSuccess)
            {
                Placeholder.Visibility = Visibility.Hidden;
                Browser.Visibility = Visibility.Visible;
            }
            BtnBack.IsEnabled = Browser.CanGoBack;
            BtnForward.IsEnabled = Browser.CanGoForward;
            var title = core.DocumentTitle;
            if (!string.IsNullOrWhiteSpace(title))
                Title = $"ToolsBoX — {title}";
        };

        BuildTree();
        UpdateStatusFromSyncInfo();

        if (File.Exists(Path.Combine(_contentsDir, "index.html")))
        {
            NavigateTo("index.html");
        }
        else
        {
            Placeholder.Text =
                "Aucun index.html à la racine de contents/.\n\n" +
                "Sélectionnez un fichier .html (lignes bleues) dans le panneau de gauche.";
        }
    }

    private void NavigateTo(string relativePath)
    {
        if (Browser.CoreWebView2 is null) return;
        var url = $"https://{VirtualHost}/" + relativePath.Replace('\\', '/').TrimStart('/');
        Browser.Source = new Uri(url);
        TxtAddress.Text = "contents\\" + relativePath.Replace('/', '\\');
    }

    private void BtnBack_Click(object sender, RoutedEventArgs e) => Browser.GoBack();

    private void BtnForward_Click(object sender, RoutedEventArgs e) => Browser.GoForward();

    private void BtnReload_Click(object sender, RoutedEventArgs e) => Browser.Reload();

    private void BtnHome_Click(object sender, RoutedEventArgs e)
    {
        if (_contentsDir is null) return;
        var home = Path.Combine(_contentsDir, "index.html");
        if (File.Exists(home)) NavigateTo("index.html");
    }

    private void BtnOpenFolder_Click(object sender, RoutedEventArgs e)
    {
        if (_contentsDir is null) return;
        try
        {
            Process.Start(new ProcessStartInfo("explorer.exe", _contentsDir) { UseShellExecute = true });
        }
        catch (Exception ex)
        {
            TxtStatus.Text = "Ouverture de l'explorateur impossible : " + ex.Message;
        }
    }

    private void MainWindow_PreviewKeyDown(object sender, KeyEventArgs e)
    {
        if (e.Key == Key.F5) { Browser.Reload(); e.Handled = true; }
        else if (Keyboard.Modifiers == ModifierKeys.Alt && e.Key == Key.Left) { Browser.GoBack(); e.Handled = true; }
        else if (Keyboard.Modifiers == ModifierKeys.Alt && e.Key == Key.Right) { Browser.GoForward(); e.Handled = true; }
    }

    private void BuildTree()
    {
        ContentTree.Items.Clear();
        if (_contentsDir is null) return;
        foreach (var node in ContentNode.Scan(_contentsDir))
            ContentTree.Items.Add(node);
    }

    private void TreeViewItem_Expanded(object sender, RoutedEventArgs e)
    {
        if (e.OriginalSource is TreeViewItem item && item.DataContext is ContentNode node && node.IsDir)
            node.EnsureChildren();
    }

    private void ContentTree_SelectedItemChanged(object sender, RoutedPropertyChangedEventArgs<object> e)
    {
        if (e.NewValue is not ContentNode node) return;

        if (node.IsHtml)
        {
            NavigateTo(node.RelativePath);
        }
        else if (node.IsDir)
        {
            node.IsExpanded = !node.IsExpanded;
        }
    }

    private void UpdateStatusFromSyncInfo()
    {
        if (_contentsDir is null) return;
        try
        {
            var syncInfo = Path.Combine(_contentsDir, ".sync-info.json");
            if (File.Exists(syncInfo))
            {
                using var doc = JsonDocument.Parse(File.ReadAllText(syncInfo));
                var root = doc.RootElement;
                var repo = root.TryGetProperty("source", out var r) ? r.GetString() : "ToolsBoXMachine";
                var sha = root.TryGetProperty("sourceSha", out var s) ? s.GetString() : "?";
                var at = root.TryGetProperty("syncedAtUtc", out var d) ? d.GetString() : "?";
                var shortSha = sha is { Length: >= 7 } ? sha[..7] : sha;
                TxtStatus.Text = $"Source : {repo} @ {shortSha} · synchronisé le {at}";
                return;
            }
        }
        catch { }

        try
        {
            var count = Directory.EnumerateFiles(_contentsDir, "*", SearchOption.AllDirectories).Count();
            TxtStatus.Text = $"contents/ : {count} fichiers";
        }
        catch { TxtStatus.Text = _contentsDir; }
    }
}