import { useState, useCallback } from 'preact/hooks';
import type { Page } from '../types';

interface UseRouterResult {
  currentPage: Page;
  navigate: (page: Page) => void;
  goBack: () => void;
}

export function useRouter(initialPage: Page = 'connect'): UseRouterResult {
  const [history, setHistory] = useState<Page[]>([initialPage]);

  const currentPage = history[history.length - 1];

  const navigate = useCallback((page: Page) => {
    setHistory((prev) => [...prev, page]);
  }, []);

  const goBack = useCallback(() => {
    setHistory((prev) => {
      if (prev.length > 1) {
        return prev.slice(0, -1);
      }
      return prev;
    });
  }, []);

  return {
    currentPage,
    navigate,
    goBack,
  };
}
