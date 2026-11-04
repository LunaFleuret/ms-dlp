const HOST_NAME = "com.msdlp.downloader";

const notifiedTaskStates = new Map();

chrome.runtime.onInstalled.addListener(() => {
  setupMenus();
  chrome.alarms.create("check_msdlp_tasks", { periodInMinutes: 0.1 });
});

chrome.runtime.onStartup.addListener(() => {
  setupMenus();
});

function setupMenus() {
  chrome.contextMenus.removeAll(() => {
    chrome.contextMenus.create({
      id: "msdlp_download_page",
      title: "この曲を ms-dlp でダウンロード (M4A / カバー / 歌詞)",
      contexts: ["page", "video", "audio"]
    });
    chrome.contextMenus.create({
      id: "msdlp_download_link",
      title: "リンク先の曲を ms-dlp でダウンロード",
      contexts: ["link"]
    });
  });
}

function sendToNativeHost(message, callback) {
  try {
    chrome.runtime.sendNativeMessage(HOST_NAME, message, (response) => {
      if (chrome.runtime.lastError) {
        if (callback) callback({ status: "error", message: chrome.runtime.lastError.message });
      } else {
        if (callback) callback(response);
      }
    });
  } catch (err) {
    if (callback) callback({ status: "error", message: String(err) });
  }
}

// Background alarm polling for badge & notifications
chrome.alarms.onAlarm.addListener((alarm) => {
  if (alarm.name === "check_msdlp_tasks") {
    pollTasksStatus();
  }
});

function pollTasksStatus(callback) {
  sendToNativeHost({ action: "get_tasks" }, (response) => {
    if (response && response.status === "success" && Array.isArray(response.tasks)) {
      const tasks = response.tasks;
      const activeCount = tasks.filter(t => t.status === "downloading" || t.status === "queued").length;

      if (activeCount > 0) {
        chrome.action.setBadgeText({ text: String(activeCount) });
        chrome.action.setBadgeBackgroundColor({ color: "#2d5fa5" });
      } else {
        chrome.action.setBadgeText({ text: "" });
      }

      tasks.forEach(task => {
        const lastStatus = notifiedTaskStates.get(task.id);
        if (lastStatus !== task.status) {
          if (task.status === "completed") {
            showNotification("ダウンロード完了", `${task.title || task.url}`);
            notifiedTaskStates.set(task.id, "completed");
          } else if (task.status === "error") {
            showNotification("ダウンロード失敗", `${task.title || task.url}\n${task.error || "エラーが発生しました"}`, true);
            notifiedTaskStates.set(task.id, "error");
          }
        }
      });
      if (callback) callback(response);
    } else {
      if (callback) callback(response);
    }
  });
}

function showNotification(title, message, isError = false) {
  chrome.notifications.create({
    type: "basic",
    iconUrl: "icons/icon128.png",
    title: title,
    message: message,
    priority: isError ? 2 : 1
  });
}

// Messages from popup
chrome.runtime.onMessage.addListener((request, _sender, sendResponse) => {
  if (request.action === "start_download_from_popup") {
    sendToNativeHost(
      {
        action: "start_download",
        url: request.url,
        title: request.title,
        options: request.options
      },
      (res) => {
        sendResponse(res);
        pollTasksStatus();
      }
    );
    return true; // Keep message channel open for async response
  } else if (request.action === "get_tasks_from_popup") {
    sendToNativeHost({ action: "get_tasks" }, (res) => {
      sendResponse(res);
    });
    return true;
  } else if (request.action === "clear_finished_tasks") {
    sendToNativeHost({ action: "clear_finished" }, (res) => {
      sendResponse(res);
    });
    return true;
  } else if (request.action === "open_output_dir") {
    sendToNativeHost({ action: "open_output_dir" }, (res) => {
      sendResponse(res);
    });
    return true;
  }
});

// Right Click Context Menus
chrome.contextMenus.onClicked.addListener((info, tab) => {
  const targetUrl = (info.menuItemId === "msdlp_download_link") ? info.linkUrl : (info.pageUrl || tab?.url || info.srcUrl);
  if (!targetUrl || !/^https?:\/\//i.test(targetUrl)) {
    return;
  }

  chrome.storage.local.get(["audioFormat", "coverMode", "embedLyrics", "parallelFragments"], (data) => {
    const options = {
      audio_format: data.audioFormat || "m4a",
      cover_mode_non_square: data.coverMode || "stretch",
      embed_lyrics: (typeof data.embedLyrics === "boolean") ? data.embedLyrics : true,
      concurrent_fragments: (data.parallelFragments !== false) ? 5 : 1
    };

    sendToNativeHost(
      {
        action: "start_download",
        url: targetUrl,
        title: tab?.title || targetUrl,
        options: options
      },
      (res) => {
        if (res && res.status === "success") {
          showNotification("ダウンロード開始", tab?.title || targetUrl);
          pollTasksStatus();
        } else {
          showNotification("ホスト未検出", "install_host.bat を実行してホストを登録してください。", true);
        }
      }
    );
  });
});
