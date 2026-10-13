import { computed, onMounted, onUnmounted, ref } from "vue";
import { getSceneSnapshot, subscribeSceneEvents, type MappingNotice, type SceneSnapshot } from "./bridge";

/** Applied mapping notices are authoritative; menu candidates and editor drafts are not. */
export function useCurrentTemplate() {
  const applied = ref<MappingNotice | null>(null);
  const sceneSnapshot = ref<SceneSnapshot | null>(null);
  const error = ref("");
  let revision = -1;
  let snapshotEvents = 0;
  let readRequest = 0;
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

  return { applied, sceneSnapshot, templateId: computed(() => applied.value?.templateId ?? null), error, refresh };
}
