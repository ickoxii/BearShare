import { useRef, useEffect } from 'preact/hooks';

interface EditorProps {
  content: string;
  onChange: (content: string) => void;
  disabled: boolean;
  roomId: string | null;
  onSync: () => void;
  onGetActivity: () => void;
}

export function Editor({ content, onChange, disabled, roomId, onSync, onGetActivity }: EditorProps) {
  const textareaRef = useRef<HTMLTextAreaElement>(null);
  const lastCursorPosRef = useRef<number>(0);

  // Save cursor position before content update
  useEffect(() => {
    if (textareaRef.current) {
      lastCursorPosRef.current = textareaRef.current.selectionStart;
    }
  }, [content]);

  // Restore cursor position after content update
  useEffect(() => {
    if (textareaRef.current && !disabled) {
      const cursorPos = Math.min(lastCursorPosRef.current, content.length);
      textareaRef.current.selectionStart = cursorPos;
      textareaRef.current.selectionEnd = cursorPos;
    }
  }, [content, disabled]);

  const handleChange = (e: Event) => {
    const target = e.target as HTMLTextAreaElement;
    onChange(target.value);
  };

  return (
    <div className="editor-panel">
      <div className="editor-header">
        <h2>Document</h2>
        <span className="room-info">
          {roomId ? `Room: ${roomId.substring(0, 8)}...` : 'Not in a room'}
        </span>
      </div>

      <textarea
        ref={textareaRef}
        className="editor-area"
        value={content}
        onInput={handleChange}
        disabled={disabled}
        placeholder="Join or create a room to start editing..."
      />

      <div className="toolbar">
        <button
          className="btn btn-secondary"
          onClick={onSync}
          disabled={disabled}
        >
          Sync
        </button>
        <button
          className="btn btn-secondary"
          onClick={onGetActivity}
          disabled={disabled}
        >
          Activity
        </button>
      </div>
    </div>
  );
}
