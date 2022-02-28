use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug, PartialEq)]
pub struct OncallList {
    pub oncalls: Vec<OncallInstance>,
}

#[derive(Serialize, Deserialize, Debug, PartialEq)]
pub struct OncallInstance {
    escalation_policy: EscalationPolicy,
    pub user: User,
    schedule: Option<Schedule>,
    pub escalation_level: u8,
    start: Option<String>,
    end: Option<String>,
}

#[derive(Serialize, Deserialize, Debug, PartialEq)]
struct EscalationPolicy {}

#[derive(Serialize, Deserialize, Debug, PartialEq)]
pub struct User {
    pub name: String,
    pub email: String,
    pub id: String,
    #[serde(rename = "type")]
    pub user_type: String,
}

#[derive(Serialize, Deserialize, Debug, PartialEq)]
struct Schedule {}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::{from_value, json, to_value};

    #[test]
    fn should_serialize_oncall_list() {
        let oncalls_list_json = json!(
            {"oncalls" : [{
                "escalation_policy": {},
                "user": {
                    "name": "receptionist bot",
                    "email": "bot@receptionist.com",
                    "id": "PS12345",
                    "type": "admin"
                },
                "schedule": {},
                "escalation_level": 1,
                "start": "any date string",
                "end": "any date string",
            }]}
        );

        let oncalls_list: OncallList = from_value(oncalls_list_json.clone()).unwrap();

        assert_eq!(to_value(oncalls_list).unwrap(), oncalls_list_json);
    }
}
