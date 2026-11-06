document.addEventListener("DOMContentLoaded", async () => {
  // Elements
  const tabNewBtn = document.getElementById("tabNewBtn");
  const tabTasksBtn = document.getElementById("tabTasksBtn");
  const viewNew = document.getElementById("viewNew");
  const viewTasks = document.getElementById("viewTasks");
  const activeTasksBadge = document.getElementById("activeTasksBadge");

  const pageTitleEl = document.getElementById("pageTitle");
  const pageUrlInput = document.getElementById("pageUrl");
  const openOptionsBtn = document.getElementById("openOptionsBtn");

  const audioFormatSelect = document.getElementById("audioFormat");
  const coverModeSelect = document.getElementById("coverMode");
  const normalizeAudioCheckbox = document.getElementById("normalizeAudio");
  const embedLyricsCheckbox = document.getElementById("embedLyrics");
  const autoCropShortsCheckbox = document.getElementById("autoCropShorts");

  const downloadBtn = document.getElementById("downloadBtn");
  const btnText = downloadBtn.querySelector(".btn-text");
  const btnSpinner = document.getElementById("btnSpinner");

  const statusBanner = document.getElementById("statusBanner");
  const statusText = document.getElementById("statusText");

  const tasksList = document.getElementById("tasksList");
  const emptyTasksState = document.getElementById("emptyTasksState");
  const tasksCountLabel = document.getElementById("tasksCountLabel");
  const openFolderBtn = document.getElementById("openFolderBtn");
  const clearFinishedBtn = document.getElementById("clearFinishedBtn");

  let pollInterval = null;
  let currentTabTitle = "";

  // HTML escape helper
  function escapeHtml(str) {
    if (!str) return "";
    return String(str)
      .replace(/&/g, "&amp;")
      .replace(/</g, "&lt;")
      .replace(/>/g, "&gt;")
      .replace(/"/g, "&quot;")
      .replace(/'/g, "&#039;");
  }

  // Tab switching
  tabNewBtn.addEventListener("click", () => switchTab("new"));
  tabTasksBtn.addEventListener("click", () => switchTab("tasks"));

  function switchTab(tabName) {
    if (tabName === "new") {
      tabNewBtn.classList.add("active");
      tabTasksBtn.classList.remove("active");
      viewNew.classList.remove("hidden");
      viewTasks.classList.add("hidden");
    } else {
      tabTasksBtn.classList.add("active");
      tabNewBtn.classList.remove("active");
      viewTasks.classList.remove("hidden");
      viewNew.classList.add("hidden");
      renderTasks();
    }
  }

  // Options page navigation
  if (openOptionsBtn) {
    openOptionsBtn.addEventListener("click", () => {
      if (chrome.runtime.openOptionsPage) {
        chrome.runtime.openOptionsPage();
      } else {
        window.open(chrome.runtime.getURL("options.html"));
      }
    });
  }

  // Load saved preferences
  chrome.storage.local.get(
    ["audioFormat", "coverMode", "normalizeAudio", "embedLyrics", "autoCropShorts"],
    (data) => {
      if (data.audioFormat) audioFormatSelect.value = data.audioFormat;
      if (data.coverMode) coverModeSelect.value = data.coverMode;
      if (typeof data.normalizeAudio === "boolean") {
        normalizeAudioCheckbox.checked = data.normalizeAudio;
      } else {
        normalizeAudioCheckbox.checked = true; // デフォルトON
      }
      if (typeof data.embedLyrics === "boolean") embedLyricsCheckbox.checked = data.embedLyrics;
      if (typeof data.autoCropShorts === "boolean") {
        autoCropShortsCheckbox.checked = data.autoCropShorts;
      } else {
        autoCropShortsCheckbox.checked = true; // デフォルトON
      }
      checkShortsAutoCrop();
    }
  );

  // Save preferences on change
  function saveCurrentPreferences() {
    chrome.storage.local.set({
      audioFormat: audioFormatSelect.value,
      coverMode: coverModeSelect.value,
      normalizeAudio: normalizeAudioCheckbox.checked,
      embedLyrics: embedLyricsCheckbox.checked,
      autoCropShorts: autoCropShortsCheckbox.checked
    });
  }

  function checkShortsAutoCrop() {
    const url = pageUrlInput.value.trim();
    if (/youtube\.com\/shorts\//i.test(url) && autoCropShortsCheckbox.checked) {
      coverModeSelect.value = "crop";
    }
  }

  audioFormatSelect.addEventListener("change", saveCurrentPreferences);
  coverModeSelect.addEventListener("change", saveCurrentPreferences);
  normalizeAudioCheckbox.addEventListener("change", saveCurrentPreferences);
  embedLyricsCheckbox.addEventListener("change", saveCurrentPreferences);
  autoCropShortsCheckbox.addEventListener("change", () => {
    saveCurrentPreferences();
    checkShortsAutoCrop();
  });
  pageUrlInput.addEventListener("input", checkShortsAutoCrop);

  // Get active tab info
  try {
    const tabs = await chrome.tabs.query({ active: true, currentWindow: true });
    if (tabs && tabs[0]) {
      const activeTab = tabs[0];
      currentTabTitle = activeTab.title || "";
      pageTitleEl.textContent = currentTabTitle || "（タイトル未取得）";

      if (activeTab.url && /^https?:\/\//i.test(activeTab.url)) {
        pageUrlInput.value = activeTab.url;
        checkShortsAutoCrop();
      } else {
        pageUrlInput.value = "";
        pageUrlInput.placeholder = "有効なURLを入力してください";
      }
    }
  } catch (e) {
    pageTitleEl.textContent = "URLを手動入力してください";
  }

  // Start Download
  downloadBtn.addEventListener("click", () => {
    const url = pageUrlInput.value.trim();
    if (!url) {
      showStatus("URLを入力してください。", "error");
      return;
    }

    setButtonLoading(true);
    hideStatus();

    let finalCoverMode = coverModeSelect.value;
    if (/youtube\.com\/shorts\//i.test(url) && autoCropShortsCheckbox.checked) {
      finalCoverMode = "crop";
    }

    const options = {
      audio_format: audioFormatSelect.value,
      cover_mode_non_square: finalCoverMode,
      normalize_audio: normalizeAudioCheckbox.checked,
      auto_crop_shorts: autoCropShortsCheckbox.checked,
      embed_lyrics: embedLyricsCheckbox.checked,
      concurrent_fragments: 5
    };

    chrome.runtime.sendMessage(
      {
        action: "start_download_from_popup",
        url: url,
        title: currentTabTitle || url,
        options: options
      },
      (response) => {
        setButtonLoading(false);
        if (response && response.status === "success") {
          showStatus("ダウンロードタスクを登録しました", "success");
          setTimeout(() => {
            switchTab("tasks");
          }, 800);
        } else {
          const errMsg = response?.message || "ダウンロードの開始に失敗しました。install_host.bat を実行してホストを登録してください。";
          showStatus(errMsg, "error");
        }
      }
    );
  });

  // Open output folder
  if (openFolderBtn) {
    openFolderBtn.addEventListener("click", () => {
      const origText = openFolderBtn.textContent;
      openFolderBtn.textContent = "開いています...";
      chrome.runtime.sendMessage({ action: "open_output_dir" }, (response) => {
        setTimeout(() => {
          openFolderBtn.textContent = origText;
        }, 1000);
        if (response && response.status === "error") {
          showStatus(response.error || response.message || "フォルダを開けませんでした。", "error");
        }
      });
    });
  }

  // Clear finished tasks
  if (clearFinishedBtn) {
    clearFinishedBtn.addEventListener("click", () => {
      chrome.runtime.sendMessage({ action: "clear_finished_tasks" }, () => {
        renderTasks();
      });
    });
  }

  // Cancel task event delegation
  if (tasksList) {
    tasksList.addEventListener("click", (e) => {
      const cancelBtn = e.target.closest(".task-cancel-btn");
      if (cancelBtn) {
        const taskId = cancelBtn.dataset.id;
        if (taskId) {
          cancelBtn.disabled = true;
          cancelBtn.textContent = "中止中...";
          chrome.runtime.sendMessage({ action: "cancel_task", id: taskId }, () => {
            renderTasks();
          });
        }
      }
    });
  }

  // Status helper
  function showStatus(msg, type = "success") {
    statusText.textContent = msg;
    statusBanner.className = `status-banner ${type}`;
    statusBanner.classList.remove("hidden");
  }

  function hideStatus() {
    statusBanner.classList.add("hidden");
  }

  function setButtonLoading(loading) {
    downloadBtn.disabled = loading;
    if (loading) {
      btnText.textContent = "登録中...";
      btnSpinner.classList.remove("hidden");
    } else {
      btnText.textContent = "音楽をダウンロード";
      btnSpinner.classList.add("hidden");
    }
  }

  // Status label translation helper
  function getStatusLabel(status) {
    switch (status) {
      case "queued": return "待機中";
      case "downloading": return "ダウンロード中";
      case "converting": return "変換・タグ埋込中";
      case "completed": return "完了";
      case "error": return "エラー";
      case "cancelled": return "キャンセル済";
      default: return status;
    }
  }

  // Render Tasks
  function renderTasks() {
    chrome.runtime.sendMessage({ action: "get_tasks_from_popup" }, (response) => {
      if (!response || response.status !== "success" || !Array.isArray(response.tasks)) {
        return;
      }

      const tasks = response.tasks;
      const activeCount = tasks.filter(t => t.status === "downloading" || t.status === "queued" || t.status === "converting").length;

      // Update badge
      if (activeCount > 0) {
        activeTasksBadge.textContent = String(activeCount);
        activeTasksBadge.classList.remove("hidden");
      } else {
        activeTasksBadge.classList.add("hidden");
      }

      tasksCountLabel.textContent = `${tasks.length} 件のタスク`;

      if (tasks.length === 0) {
        emptyTasksState.classList.remove("hidden");
        const existingCards = tasksList.querySelectorAll(".task-card");
        existingCards.forEach(c => c.remove());
        return;
      }

      emptyTasksState.classList.add("hidden");

      const currentIds = new Set(tasks.map(t => t.id));
      const existingCards = tasksList.querySelectorAll(".task-card");
      existingCards.forEach(card => {
        if (!currentIds.has(card.dataset.id)) {
          card.remove();
        }
      });

      tasks.forEach(task => {
        let card = tasksList.querySelector(`.task-card[data-id="${task.id}"]`);
        if (!card) {
          card = document.createElement("div");
          card.className = "task-card";
          card.dataset.id = task.id;
          tasksList.appendChild(card);
        }

        const isRunning = task.status === "downloading" || task.status === "queued" || task.status === "converting";
        const rawPercent = task.percent !== undefined ? task.percent : task.progress;
        const percent = Math.min(100, Math.max(0, parseFloat(rawPercent) || 0));

        let progressClass = "progress-fill";
        if (task.status === "converting") progressClass += " converting";
        if (task.status === "completed") progressClass += " completed";
        if (task.status === "error") progressClass += " error";

        let errorHtml = "";
        if (task.error) {
          errorHtml = `
            <div class="task-error-msg">
              ${escapeHtml(task.error)}
            </div>
          `;
        }

        card.innerHTML = `
          <div class="task-card-header">
            <div class="task-title" title="${escapeHtml(task.title || task.url)}">
              ${escapeHtml(task.title || task.url)}
            </div>
            <span class="task-badge ${task.status}">${getStatusLabel(task.status)}</span>
          </div>

          <div class="progress-container">
            <div class="${progressClass}" style="width: ${percent}%;"></div>
          </div>

          <div class="task-meta">
            <div class="task-stats">
              <span><strong>${percent.toFixed(1)}%</strong></span>
              ${task.total_size && task.total_size !== "Unknown" ? `<span>(${escapeHtml(task.total_size)})</span>` : ""}
              ${task.status === "completed" ? `<span>完了</span>` : ""}
              ${task.speed && task.status !== "completed" ? `<span class="task-speed">${escapeHtml(task.speed)}</span>` : ""}
              ${task.eta && task.eta !== "--:--" ? `<span>残り: ${escapeHtml(task.eta)}</span>` : ""}
            </div>
            ${isRunning ? `<button class="task-cancel-btn" data-action="cancel" data-id="${task.id}">キャンセル</button>` : ""}
          </div>

          ${errorHtml}
        `;
      });
    });
  }

  // Initial task render and auto-refresh interval (700ms)
  renderTasks();
  pollInterval = setInterval(renderTasks, 700);

  // Switch to tasks tab automatically if tasks are already running
  chrome.runtime.sendMessage({ action: "get_tasks_from_popup" }, (response) => {
    if (response && response.status === "success" && Array.isArray(response.tasks)) {
      const active = response.tasks.filter(t => t.status === "downloading" || t.status === "queued" || t.status === "converting");
      if (active.length > 0) {
        switchTab("tasks");
      }
    }
  });

  window.addEventListener("unload", () => {
    if (pollInterval) clearInterval(pollInterval);
  });
});
