import { AdminUserDetailView } from "@/components/admin/user-detail-view";

type Props = { params: Promise<{ id: string }> };

export default async function AdminUserDetailPage({ params }: Props) {
  const { id } = await params;
  return <AdminUserDetailView userId={decodeURIComponent(id)} />;
}
