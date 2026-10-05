<template>
  <div class="h-full min-h-full">
    <Milkdown />
  </div>
</template>

<script setup lang="ts">
import { convertFileSrc } from '@tauri-apps/api/core';
import { Milkdown, useEditor } from '@milkdown/vue';
import { Crepe } from '@milkdown/crepe';

import { commandsCtx, editorViewCtx } from '@milkdown/kit/core';
import { imageBlockConfig, imageBlockSchema } from '@milkdown/kit/component/image-block';
import { addBlockTypeCommand } from '@milkdown/kit/preset/commonmark';
import type { Ctx } from '@milkdown/kit/ctx';
import { Plugin, PluginKey, TextSelection } from '@milkdown/kit/prose/state';
import type { Node as ProseNode } from '@milkdown/kit/prose/model';
import type { EditorView } from '@milkdown/kit/prose/view';
import { $prose } from '@milkdown/kit/utils';

import { invoke } from '@tauri-apps/api/core';

import { useCurrentNoteStore } from '../../stores/currentNoteStore';
import { useToast } from 'vue-toastification';
import { isError } from '../../lib/errors';
import { createGoogleProvider } from './googleAiProvider';
import { computed } from 'vue';
import { useUserConfigStore } from '../../stores/userConfig';
const userConfig = useUserConfigStore();
const aiFeaturesOn = computed(() => {
  return userConfig.config['online.aiFeatures'] == 'on';
});
const props = defineProps<{ defaultValue: string; noteId?: string }>();

const emit = defineEmits<{
  (e: 'change', content: string): void;
}>();
const toast = useToast();
const currentNoteStore = useCurrentNoteStore();

function countWords(text: string): number {
  const cleaned = text.trim();

  if (!cleaned) return 0;

  return cleaned.split(/\s+/).length;
}

const MAX_SYNCED_ATTACHMENT_BYTES = 20 * 1024 * 1024;

/**
 * Stores an image as an attachment of the open note and returns the
 * `attachment://` URL the editor should reference. Shows a toast and rethrows
 * on failure.
 */
async function uploadImage(file: File): Promise<string> {
  const bytes = new Uint8Array(await file.arrayBuffer());
  if (bytes.length > MAX_SYNCED_ATTACHMENT_BYTES) {
    toast.warning('Attachments over 20mb can not be synced', {
      timeout: 10000,
    });
  }
  try {
    const attachmentId = await invoke<string>('create_attachment', {
      file: Array.from(bytes),
      fileName: file.name,
      mimeType: file.type,
      noteId: props.noteId,
    });

    return convertFileSrc(attachmentId, 'attachment');
  } catch (err) {
    reportAttachmentError(err);
    throw err;
  }
}

function reportAttachmentError(err: unknown) {
  if (isError(err, 'InvalidMimeType')) {
    toast.warning('Invalid attachment type');
  } else {
    toast.error('Failed to add image');
  }
}

const LOCAL_IMAGE_EXTENSION = /\.(png|jpe?g|webp|gif)$/i;

/**
 * Path of a local image given as a `file://` URI (`file:///home/me/a%20b.png`
 * -> `/home/me/a b.png`) or as a plain absolute path; null for anything else.
 */
function localImagePath(line: string): { path: string; isUri: boolean } | null {
  if (/^file:/i.test(line)) {
    try {
      let path = decodeURIComponent(new URL(line).pathname);
      if (/^\/[A-Za-z]:\//.test(path)) path = path.slice(1);
      return LOCAL_IMAGE_EXTENSION.test(path) ? { path, isUri: true } : null;
    } catch {
      return null;
    }
  }

  const isAbsolutePath = line.startsWith('/') || /^[A-Za-z]:[\\/]/.test(line);
  return isAbsolutePath && LOCAL_IMAGE_EXTENSION.test(line) ? { path: line, isUri: false } : null;
}

/** The URIs / paths a transfer lists: `text/uri-list` or `text/plain`, else the links in `text/html`. */
function transferLines(data: DataTransfer): string[] {
  const text = data.getData('text/uri-list') || data.getData('text/plain');
  if (text.trim()) {
    return text
      .split(/\r?\n/)
      .map((line) => line.trim())
      .filter((line) => line !== '' && !line.startsWith('#'));
  }

  // WebKitGTK leaves `text/uri-list` empty for files dragged out of a file
  // manager; the files are only listed as links in `text/html`.
  const html = data.getData('text/html');
  if (!html) return [];
  const doc = new DOMParser().parseFromString(html, 'text/html');
  return Array.from(doc.querySelectorAll('a[href]'))
    .map((link) => link.getAttribute('href')?.trim() ?? '')
    .filter((href) => href !== '');
}

/**
 * Image files copied or dragged out of a file manager reach the webview as
 * `file://` URIs or plain paths. Returns them, or null unless the list is made
 * up only of local image paths, so ordinary text is never hijacked.
 */
function localImagePaths(lines: string[]): { path: string; isUri: boolean }[] | null {
  if (lines.length === 0) return null;
  const parsed = lines.map(localImagePath);
  return parsed.every((entry) => entry !== null) ? parsed : null;
}

const IMAGE_MIME_BY_EXTENSION: Record<string, string> = {
  png: 'image/png',
  jpg: 'image/jpeg',
  jpeg: 'image/jpeg',
  webp: 'image/webp',
  gif: 'image/gif',
};

function mimeFromFileName(name: string): string {
  return IMAGE_MIME_BY_EXTENSION[name.split('.').pop()?.toLowerCase() ?? ''] ?? '';
}

/** The webview may leave `type` empty for dropped files; fill it in from the name. */
function asImageFile(file: File): File | null {
  if (file.type.startsWith('image/')) return file;
  const mime = mimeFromFileName(file.name);
  return mime ? new File([file], file.name, { type: mime }) : null;
}

/** Reads a local image (given by path) into a `File`. */
async function localImageToFile(path: string): Promise<File> {
  const bytes = await invoke<ArrayBuffer>('read_local_image', { path });
  const fileName = path.split(/[\\/]/).pop() || 'image';
  return new File([bytes], fileName, { type: mimeFromFileName(fileName) });
}

async function imageSrcToFile(src: string, index: number): Promise<File> {
  const blob = await (await fetch(src)).blob();
  const extension = blob.type.split('/')[1]?.split('+')[0] || 'png';
  return new File([blob], `pasted-image-${index + 1}.${extension}`, { type: blob.type });
}

/** Where the bytes of one pasted/dropped image come from. */
type ImageSource =
  | { kind: 'file'; file: File }
  | { kind: 'src'; src: string }
  | { kind: 'path'; path: string; isUri: boolean };

/**
 * The images a paste or drop carries, or null when it is not an image
 * transfer (then the editor's normal handling runs):
 * - image files (screenshots, files dropped by the OS),
 * - `file://` URIs / paths (WebKitGTK gives only these for files from a file
 *   manager, never the bytes),
 * - pasted HTML made of nothing but base64 `data:` images.
 * A paste whose HTML also has text (a copied table, formatted text) is left to
 * the normal paste even when it carries an image rendering of it.
 */
function imageSources(data: DataTransfer, isPaste: boolean): ImageSource[] | null {
  const html = isPaste ? data.getData('text/html') : '';
  const htmlDoc = html ? new DOMParser().parseFromString(html, 'text/html') : null;
  if (htmlDoc && (htmlDoc.body.textContent ?? '').trim() !== '') return null;

  const files = Array.from(data.files)
    .map(asImageFile)
    .filter((file): file is File => file !== null);
  if (files.length > 0) return files.map((file) => ({ kind: 'file', file }));

  const local = localImagePaths(transferLines(data));
  if (local) return local.map(({ path, isUri }) => ({ kind: 'path', path, isUri }));

  if (!htmlDoc) return null;
  const srcs = Array.from(htmlDoc.querySelectorAll('img')).map(
    (img) => img.getAttribute('src') ?? ''
  );
  const readable = srcs.length > 0 && srcs.every((src) => src.startsWith('data:image/'));
  // Remote (http) images keep the normal linked-image paste.
  return readable ? srcs.map((src) => ({ kind: 'src', src })) : null;
}

function findNodePos(view: EditorView, target: ProseNode): number | null {
  let found: number | null = null;
  view.state.doc.descendants((node, pos) => {
    if (found !== null) return false;
    if (node === target) found = pos;
    return found === null;
  });
  return found;
}

/**
 * Inserts pasted/dropped images exactly like the `/image` command plus picking
 * a file in its upload field: `addBlockTypeCommand` puts an empty image block at
 * the position, then the image block's configured `onUpload` stores the file
 * (`create_attachment`) and the block's `src` is set to the returned
 * `attachment://` URL. A block whose upload fails is removed again.
 */
async function insertImages(ctx: Ctx, view: EditorView, sources: ImageSource[], pos: number) {
  const files: File[] = [];
  for (const [index, source] of sources.entries()) {
    try {
      if (source.kind === 'file') files.push(source.file);
      else if (source.kind === 'src') files.push(await imageSrcToFile(source.src, index));
      else files.push(await localImageToFile(source.path));
    } catch (err) {
      if (view.isDestroyed) return;
      if (source.kind === 'path' && !source.isUri) {
        // Pasted as text and not a readable image: keep it as text.
        view.dispatch(view.state.tr.insertText(source.path));
      } else {
        reportAttachmentError(err);
      }
    }
  }
  if (files.length === 0 || view.isDestroyed) return;

  const doc = view.state.doc;
  view.dispatch(
    view.state.tr.setSelection(TextSelection.near(doc.resolve(Math.min(pos, doc.content.size))))
  );

  const blockType = imageBlockSchema.type(ctx);
  const commands = ctx.get(commandsCtx);
  const blocks = files.map(() => blockType.create());
  commands.call(addBlockTypeCommand.key, { nodeType: blocks[0] });
  // The inserted block ends up selected, so running the command again would
  // replace it: put every further image right after the previous one.
  for (let i = 1; i < blocks.length; i++) {
    const previous = blocks[i - 1]!;
    const previousPos = findNodePos(view, previous);
    if (previousPos === null) break;
    view.dispatch(view.state.tr.insert(previousPos + previous.nodeSize, blocks[i]!));
  }

  const { onUpload } = ctx.get(imageBlockConfig.key);
  await Promise.all(
    files.map(async (file, index) => {
      const block = blocks[index]!;
      // Not inserted (the position does not allow a block): store nothing.
      if (findNodePos(view, block) === null) return;

      let url = '';
      try {
        url = await onUpload(file);
      } catch {
        // onUpload already told the user
      }
      if (view.isDestroyed) return;

      const blockPos = findNodePos(view, block);
      if (blockPos === null) return;

      view.dispatch(
        url
          ? view.state.tr.setNodeAttribute(blockPos, 'src', url)
          : view.state.tr.delete(blockPos, blockPos + block.nodeSize)
      );
    })
  );
}

/**
 * WebKitGTK (Linux) hands the page an empty `clipboardData` when the clipboard
 * holds an image or copied files, and would paste the image itself as a
 * temporary `blob:` image. Read such a clipboard natively instead: copied
 * image files first, then image data (a screenshot).
 */
async function pasteFromNativeClipboard(ctx: Ctx, view: EditorView, pos: number) {
  try {
    const uris = await invoke<string[]>('clipboard_file_uris');
    if (uris.length > 0) {
      const local = localImagePaths(uris);
      if (local) {
        await insertImages(
          ctx,
          view,
          local.map(({ path, isUri }) => ({ kind: 'path', path, isUri })),
          pos
        );
      } else {
        toast.warning('Invalid attachment type');
      }
      return;
    }

    const png = await invoke<ArrayBuffer>('clipboard_image_png');
    if (png.byteLength === 0) return;
    const file = new File([png], 'pasted-image.png', { type: 'image/png' });
    await insertImages(ctx, view, [{ kind: 'file', file }], pos);
  } catch (err) {
    reportAttachmentError(err);
  }
}

/**
 * Routes image pastes and drops into {@link insertImages}. These are DOM-level
 * handlers on purpose: Crepe's drop indicator and Milkdown's upload/clipboard
 * plugins implement `handleDrop`/`handlePaste` and run first (the drop
 * indicator inserts any dropped text, such as a `file://` URI).
 */
const imagePastePlugin = $prose(
  (ctx) =>
    new Plugin({
      key: new PluginKey('LLAVA_IMAGE_PASTE'),
      props: {
        handleDOMEvents: {
          paste: (view, event) => {
            const data = event.clipboardData;
            if (!data || (data.types.length === 0 && data.files.length === 0)) {
              event.preventDefault();
              void pasteFromNativeClipboard(ctx, view, view.state.selection.from);
              return true;
            }

            const sources = imageSources(data, true);
            if (!sources) return false;

            event.preventDefault();
            void insertImages(ctx, view, sources, view.state.selection.from);
            return true;
          },
          drop: (view, event) => {
            // Moving content inside the editor is a normal drop.
            if (view.dragging || !event.dataTransfer) return false;

            const sources = imageSources(event.dataTransfer, false);
            if (!sources) return false;

            event.preventDefault();
            const pos =
              view.posAtCoords({ left: event.clientX, top: event.clientY })?.pos ??
              view.state.selection.from;
            void insertImages(ctx, view, sources, pos);
            return true;
          },
        },
      },
    })
);

useEditor((root) => {
  const crepe = new Crepe({
    root,

    defaultValue: props.defaultValue,

    features: {
      [Crepe.Feature.TopBar]: true,

      [Crepe.Feature.AI]: aiFeaturesOn.value,
    },

    featureConfigs: {
      [Crepe.Feature.AI]: {
        provider: createGoogleProvider(),
      },

      [Crepe.Feature.ImageBlock]: {
        onUpload: uploadImage,
      },
    },
  });

  crepe.editor.use(imagePastePlugin);

  crepe.on((listener) => {
    listener.markdownUpdated((_, markdown) => {
      emit('change', markdown);
    });

    // Same count on load and after every edit (the page used to count markdown
    // tokens first and plain text later, so the number jumped on first edit).
    const updateWordCount = (view: EditorView | undefined) => {
      if (!view) return;
      const doc = view.state.doc;
      currentNoteStore.words = countWords(doc.textBetween(0, doc.content.size, ' ', ' '));
    };

    listener.mounted((ctx) => updateWordCount(ctx.get(editorViewCtx)));
    listener.updated((ctx) => updateWordCount(ctx.get(editorViewCtx)));
  });

  return crepe;
});
</script>
