import type { WasmBridge } from '@/core/wasm-bridge';
import type { DocumentPosition } from '@/core/types';
import type { EditCommand } from './command';

function discardAll(stack: EditCommand[], wasm: WasmBridge): void {
  for (const cmd of stack) {
    cmd.discard?.(wasm);
  }
}

/** Undo/Redo 히스토리 관리 */
export class CommandHistory {
  private undoStack: EditCommand[] = [];
  private redoStack: EditCommand[] = [];
  private maxSize = 1000;

  /** 명령 실행 후 히스토리에 기록하고 실행 후 커서 위치를 반환한다. */
  execute(command: EditCommand, wasm: WasmBridge): DocumentPosition {
    const cursorAfter = command.execute(wasm);

    if (this.undoStack.length > 0) {
      const last = this.undoStack[this.undoStack.length - 1];
      const merged = last.mergeWith(command);
      if (merged) {
        this.undoStack[this.undoStack.length - 1] = merged;
        discardAll(this.redoStack, wasm);
        this.redoStack = [];
        return cursorAfter;
      }
    }

    this.undoStack.push(command);
    discardAll(this.redoStack, wasm);
    this.redoStack = [];

    if (this.undoStack.length > this.maxSize) {
      const evicted = this.undoStack.shift();
      evicted?.discard?.(wasm);
    }

    return cursorAfter;
  }

  /** Undo 성공 시 커서 위치를 반환하고, 스택이 비었으면 null을 반환한다. */
  undo(wasm: WasmBridge): DocumentPosition | null {
    const command = this.undoStack.pop();
    if (!command) return null;

    const cursorAfter = command.undo(wasm);
    this.redoStack.push(command);
    return cursorAfter;
  }

  /** Redo 성공 시 커서 위치를 반환하고, 스택이 비었으면 null을 반환한다. */
  redo(wasm: WasmBridge): DocumentPosition | null {
    const command = this.redoStack.pop();
    if (!command) return null;

    const cursorAfter = command.execute(wasm);
    this.undoStack.push(command);
    return cursorAfter;
  }

  peekUndoType(): string | null {
    return this.undoStack[this.undoStack.length - 1]?.type ?? null;
  }

  peekRedoType(): string | null {
    return this.redoStack[this.redoStack.length - 1]?.type ?? null;
  }

  /** execute() 없이 히스토리에만 기록한다. IME compositionend처럼 이미 문서에 반영된 작업에 사용한다. */
  recordWithoutExecute(command: EditCommand, wasm?: WasmBridge): void {
    if (this.undoStack.length > 0) {
      const last = this.undoStack[this.undoStack.length - 1];
      const merged = last.mergeWith(command);
      if (merged) {
        this.undoStack[this.undoStack.length - 1] = merged;
        if (wasm) {
          discardAll(this.redoStack, wasm);
        }
        this.redoStack = [];
        return;
      }
    }

    this.undoStack.push(command);
    if (wasm) {
      discardAll(this.redoStack, wasm);
    }
    this.redoStack = [];

    if (this.undoStack.length > this.maxSize) {
      const evicted = this.undoStack.shift();
      if (wasm) evicted?.discard?.(wasm);
    }
  }

  canUndo(): boolean { return this.undoStack.length > 0; }
  canRedo(): boolean { return this.redoStack.length > 0; }

  /** 문서 로드 시 히스토리와 스냅샷 리소스를 정리한다. */
  clear(wasm?: WasmBridge): void {
    if (wasm) {
      discardAll(this.undoStack, wasm);
      discardAll(this.redoStack, wasm);
    }
    this.undoStack = [];
    this.redoStack = [];
  }
}
