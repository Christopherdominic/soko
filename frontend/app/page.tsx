import { SystemStatus } from "@/components/SystemStatus";

export default function Home() {
  return (
    <main className="mx-auto w-full max-w-md flex-1 px-4 py-8">
      <header className="mb-8 rounded-2xl bg-forest px-6 py-8 text-ivory">
        <p className="text-sm uppercase tracking-widest text-lime">Soko</p>
        <h1 className="mt-2 text-3xl font-extrabold">Every market. Every payment.</h1>
        <p className="mt-2 text-ivory/80">Smarter payments for everyday business.</p>
      </header>

      <section>
        <h2 className="mb-3 text-lg font-bold">System status</h2>
        <SystemStatus />
      </section>
    </main>
  );
}
