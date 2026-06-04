import type { ProfileDashboardData } from "../../components/profile/types";

export type SquadViewMode = "no_squad" | "leader" | "member";

export type SquadSectionProps = {
  data: ProfileDashboardData;
  authToken: string;
  onChanged: () => Promise<void>;
};
