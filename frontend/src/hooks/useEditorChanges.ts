import { useCallback, useRef, useEffect } from 'preact/hooks';

interface UseEditorChangesOptions {
  documentContent: string;
  onInsert: (position: number, text: string) => void;
  onDelete: (position: number, length: number) => void;
  debounceMs?: number;
}

export function useEditorChanges({
  documentContent,
  onInsert,
  onDelete,
  debounceMs = 1,
}: UseEditorChangesOptions) {
  const lastContentRef = useRef<string>(documentContent);
  const timeoutRef = useRef<number>();
  const isUpdatingRef = useRef<boolean>(false);

  // Update lastContent when documentContent changes from external source
  useEffect(() => {
    if (!isUpdatingRef.current) {
      lastContentRef.current = documentContent;
    }
  }, [documentContent]);

  const handleChange = useCallback((newContent: string) => {
    if (isUpdatingRef.current) return;

    // Clear existing timeout
    if (timeoutRef.current) {
      clearTimeout(timeoutRef.current);
    }

    // Debounce the change
    timeoutRef.current = window.setTimeout(() => {
      const oldContent = lastContentRef.current;

      // Find the change
      let start = 0;
      while (start < oldContent.length && start < newContent.length && oldContent[start] === newContent[start]) {
        start++;
      }

      let oldEnd = oldContent.length;
      let newEnd = newContent.length;
      while (oldEnd > start && newEnd > start && oldContent[oldEnd - 1] === newContent[newEnd - 1]) {
        oldEnd--;
        newEnd--;
      }

      // Handle deletion
      if (oldEnd > start) {
        onDelete(start, oldEnd - start);
      }

      // Handle insertion
      if (newEnd > start) {
        onInsert(start, newContent.substring(start, newEnd));
      }

      lastContentRef.current = newContent;
    }, debounceMs);
  }, [onInsert, onDelete, debounceMs]);

  // Cleanup timeout on unmount
  useEffect(() => {
    return () => {
      if (timeoutRef.current) {
        clearTimeout(timeoutRef.current);
      }
    };
  }, []);

  return { handleChange };
}
