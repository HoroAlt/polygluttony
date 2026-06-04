interface PageHeaderProps {
  title: string;
  description?: string;
}

/** Consistent screen heading used by each feature page. */
export function PageHeader({ title, description }: PageHeaderProps) {
  return (
    <div className="flex flex-col gap-1">
      <h1 className="text-2xl font-semibold tracking-tight">{title}</h1>
      {description ? (
        <p className="max-w-prose text-sm text-muted-foreground">
          {description}
        </p>
      ) : null}
    </div>
  );
}
