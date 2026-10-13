import { computed, onMounted, onUnmounted, ref } from "vue";
import { getSceneSnapshot, selectCurrentTemplate, subscribeSceneEvents, type MappingNotice, type SceneSnapshot } from "./bridge";

/** Applied mapping notices are authoritative; menu candidates and editor drafts are not. */
export function useCurrentTemplate() {
  const applied = ref<MappingNotice | null>(null);
  const sceneSnapshot = ref<SceneSnapshot | null>(null);
  const error = ref("");
  const applying = ref(false);
  let revision = -1;
  let snapshotEvents = 0;
  let readRequest = 0;
  let applyRequest = 0;
  let disposed = false;
  let unlisten: (() => void) | undefined;

  function acceptNotice(notice: MappingNotice | null, nextRevision: number) {
    if (disposed || !notice || nextRevision < revision) return;
    applied.value = notice;
    revision = nextRevision;
  }
  function acceptSnapshot(snapshot: SceneSnapshot | null) {
    if (disposed || !snapshot) return;
    sceneSnapshot.value = snapshot;
    acceptNotice(snapshot.mappingNotice, snapshot.mappingNoticeRevision);
  }
  async function refresh() {
    const request = ++readRequest;
    const eventsAtRequest = snapshotEvents;
    try {
      const snapshot = await getSceneSnapshot();
      if (disposed || request !== readRequest) return;
      // Physical edges can share the same scene generation. A read started
      // before an event must never overwrite the newer pressed/released state.
      if (snapshotEvents === eventsAtRequest) acceptSnapshot(snapshot);
      else if (snapshot) acceptNotice(snapshot.mappingNotice, snapshot.mappingNoticeRevision);
      error.value = "";
    } catch (reason) {
      if (!disposed && request === readRequest) error.value = reason instanceof Error ? reason.message : String(reason);
    }
  }

  async function activate(templateId: string | null): Promise<void> {
    if (disposed || applying.value) return;
    applying.value = true;
    error.value = "";
    const request = ++applyRequest;
    const revisionAtStart = revision;
    const eventsAtRequest = snapshotEvents;
    try {
      const snapshot = await selectCurrentTemplate(templateId);
      if (disposed || request !== applyRequest) return;
      const notice = snapshot.mappingNotice;
      // MappingNotice.kind describes action availability (direct/common/disabled/unconfigured),
      // not the selected-template category. The notice's templateId is the applied identity.
      const matchesRequest = notice && notice.templateId === templateId;
      if (!matchesRequest) throw new Error("模板应用未得到对应的引擎确认。");
      if (snapshotEvents === eventsAtRequest && snapshot.mappingNoticeRevision >= revision) acceptSnapshot(snapshot);
      else if (snapshot.mappingNoticeRevision > revision) acceptNotice(notice, snapshot.mappingNoticeRevision);
    } catch (reason) {
      if (!disposed && request === applyRequest && revision === revisionAtStart)
        error.value = reason instanceof Error ? reason.message : String(reason);
    } finally {
      if (!disposed && request === applyRequest) applying.value = false;
    }
  }

  onMounted(async () => {
    try {
      const stop = await subscribeSceneEvents(event => {
        if (event.type === "mapping_applied") acceptNotice(event.notice, event.revision);
        else if (event.type === "snapshot") {
          ++snapshotEvents;
          acceptSnapshot(event.snapshot);
        }
      });
      if (disposed) { stop(); return; }
      unlisten = stop;
    } catch {
      if (!disposed) error.value = "无法接收模板变化，请重新打开页面。";
      return;
    }
    if (!disposed) await refresh();
  });
  onUnmounted(() => { disposed = true; unlisten?.(); });

  const persistenceMessage = computed(() => {
    return applied.value?.manualSaveStatus === "failed"
      ? "当前模板已应用，但未能记住本次选择；重启后将使用之前记住的选择（如无记录则为通用配置）。"
      : "";
  });
  return { applied, sceneSnapshot, templateId: computed(() => applied.value?.templateId ?? null), error, applying, persistenceMessage, refresh, activate };
}
