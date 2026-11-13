"use client";

import Link from "next/link";
import { ArrowRight, QrCode } from "lucide-react";
import { useMerchant } from "@/hooks/useMerchant";
import { greeting } from "@/lib/format";
import { ButtonLink, Card, CardHeader, PageHeader } from "@/components/ui";
import { DashboardStats } from "@/components/DashboardStats";
import { RecentPayments } from "@/components/RecentPayments";
import { SystemStatus } from "@/components/SystemStatus";

export function DashboardView() {
  const merchant = useMerchant();
  const today = new Date().toLocaleDateString(undefined, { weekday: "long", day: "numeric", month: "long" });

  return (
    <>
      <PageHeader
        title={greeting()}
        subtitle={
          <>
            <span className="hidden sm:inline">{merchant.business_name} · </span>
            {today}
          </>
        }
        action={
          <span className="hidden sm:block">
            <ButtonLink href="/receive" size="lg">
              <QrCode aria-hidden className="h-5 w-5" /> Receive payment
            </ButtonLink>
          </span>
        }
      />

      <div className="space-y-6">
        <DashboardStats />

        <Card>
          <CardHeader
            title="Recent transactions"
            action={
              <Link href="/history" className="inline-flex items-center gap-1 text-sm font-semibold text-forest hover:underline">
                View all <ArrowRight aria-hidden className="h-4 w-4" />
              </Link>
            }
          />
          <RecentPayments limit={5} />
        </Card>

        <Card className="px-5 py-4">
          <SystemStatus />
        </Card>
      </div>
    </>
  );
}
