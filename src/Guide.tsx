import guide from "./guide.json";
import type { Config } from "./types";

export default function Guide({
  config,
  onNavigate,
}: {
  config: Config;
  onNavigate: (page: string) => void;
}) {
  return (
    <article className="precision-guide" aria-label={guide.title}>
      <section className="card">
        <h2>{guide.title}</h2>
        <p>{guide.intro}</p>
        <p className="guide-current">
          Current settings draft: Precision step {config.precision.step} px;
          Normal starting speed {config.normal.min_speed} px/s; acceleration{" "}
          {config.features.acceleration.enabled
            ? config.features.acceleration.strategy
            : "original power curve"}
          .
          {config.precision.step > 1 &&
            " Set the Precision step to 1 for one-pixel aiming."}
        </p>
        <nav aria-label="Guide contents" className="guide-contents">
          {guide.sections.map((section) => (
            <a key={section.id} href={`#guide-${section.id}`}>
              {section.title}
            </a>
          ))}
        </nav>
      </section>
      {guide.sections.map((section) => (
        <section
          className="card"
          id={`guide-${section.id}`}
          key={section.id}
          aria-labelledby={`guide-heading-${section.id}`}
        >
          <div className="section-head">
            <h2 id={`guide-heading-${section.id}`}>{section.title}</h2>
            {section.page && (
              <button onClick={() => onNavigate(section.page!)}>
                Open {section.page}
              </button>
            )}
          </div>
          {section.paragraphs?.map((paragraph) => (
            <p key={paragraph}>{paragraph}</p>
          ))}
          {section.steps && (
            <ol>
              {section.steps.map((step) => (
                <li key={step}>{step}</li>
              ))}
            </ol>
          )}
          {section.table && (
            <div className="guide-table-wrap">
              <table>
                <caption>{section.title}</caption>
                <thead>
                  <tr>
                    {section.table.columns.map((column) => (
                      <th key={column} scope="col">
                        {column}
                      </th>
                    ))}
                  </tr>
                </thead>
                <tbody>
                  {section.table.rows.map((row) => (
                    <tr key={row[0]}>
                      {row.map((cell, i) =>
                        i === 0 ? (
                          <th scope="row" key={i}>
                            {cell}
                          </th>
                        ) : (
                          <td key={i}>{cell}</td>
                        ),
                      )}
                    </tr>
                  ))}
                </tbody>
              </table>
            </div>
          )}
        </section>
      ))}
    </article>
  );
}
