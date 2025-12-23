import type { Version } from '../types';

interface VersionListProps {
  versions: Version[];
  onRestore: (seq: number) => void;
}

export function VersionList({ versions, onRestore }: VersionListProps) {
  if (versions.length === 0) {
    return null;
  }

  const handleRestore = (seq: number) => {
    if (confirm(`Restore to version ${seq}? This will replace the current document.`)) {
      onRestore(seq);
    }
  };

  return (
    <div className="version-list">
      <h4>Saved Versions</h4>
      {versions.map((version) => (
        <div
          key={version.seq}
          className="version-item"
          onClick={() => handleRestore(version.seq)}
        >
          <div className="version-id">Version {version.seq}</div>
          <div className="version-meta">
            {version.author || 'Anonymous'} • {new Date(version.timestamp).toLocaleString()}
          </div>
        </div>
      ))}
    </div>
  );
}
