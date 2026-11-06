import type { Source } from '../types';

interface EditorHeadingProps {
  source: Source | undefined;
  chosen: Source[];
  totalRegions: number;
}

export function EditorHeading({ source, chosen, totalRegions }: EditorHeadingProps) {
  return (
    <section className="page-heading">
      <div>
        <h1>
          {source
            ? chosen.length > 1
              ? chosen.length + ' sources selected'
              : source.title
            : 'Media Utility'}
        </h1>
      </div>
      {source && (
        <span className="pill">
          {totalRegions} {totalRegions === 1 ? 'output' : 'outputs'}
        </span>
      )}
    </section>
  );
}
