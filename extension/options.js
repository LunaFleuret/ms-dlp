document.addEventListener("DOMContentLoaded", () => {
  const defaultAudioFormat = document.getElementById("defaultAudioFormat");
  const defaultCoverMode = document.getElementById("defaultCoverMode");
  const defaultEmbedLyrics = document.getElementById("defaultEmbedLyrics");
  const defaultAutoCropShorts = document.getElementById("defaultAutoCropShorts");
  const defaultParallel = document.getElementById("defaultParallel");

  const testConnBtn = document.getElementById("testConnBtn");
  const connResult = document.getElementById("connResult");
  const saveBtn = document.getElementById("saveBtn");
  const saveStatus = document.getElementById("saveStatus");

  // Load saved settings
  chrome.storage.local.get([
    "audioFormat",
    "coverMode",
    "embedLyrics",
    "autoCropShorts",
    "parallelFragments"
  ], (items) => {
    if (items.audioFormat) defaultAudioFormat.value = items.audioFormat;
    if (items.coverMode) defaultCoverMode.value = items.coverMode;
    if (typeof items.embedLyrics === "boolean") defaultEmbedLyrics.checked = items.embedLyrics;
    if (typeof items.autoCropShorts === "boolean") {
      defaultAutoCropShorts.checked = items.autoCropShorts;
    } else {
      defaultAutoCropShorts.checked = true;
    }
    if (typeof items.parallelFragments === "boolean") defaultParallel.checked = items.parallelFragments;
  });

  // Test Host Connection
  testConnBtn.addEventListener("click", () => {
    connResult.className = "conn-result";
    connResult.style.display = "block";
    connResult.textContent = "接続確認中...";

    chrome.runtime.sendMessage({ action: "get_tasks_from_popup" }, (response) => {
      if (response && response.status === "success") {
        connResult.className = "conn-result success";
        connResult.textContent = "[OK] ネイティブホスト（ms-dlp）との通信に成功しました。";
      } else {
        connResult.className = "conn-result error";
        const err = (response && (response.message || response.error)) ? (response.message || response.error) : "ホストとの通信に失敗しました。install_host.bat を実行してホストを登録してください。";
        connResult.textContent = `[Error] ${err}`;
      }
    });
  });

  // Save Settings
  saveBtn.addEventListener("click", () => {
    const configData = {
      audioFormat: defaultAudioFormat.value,
      coverMode: defaultCoverMode.value,
      embedLyrics: defaultEmbedLyrics.checked,
      autoCropShorts: defaultAutoCropShorts.checked,
      parallelFragments: defaultParallel.checked
    };

    chrome.storage.local.set(configData, () => {
      saveStatus.classList.add("show");
      setTimeout(() => {
        saveStatus.classList.remove("show");
      }, 2500);
    });
  });
});

